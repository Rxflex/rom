mod client;
mod proxy;
mod socks;
mod tls;

use serde::{Deserialize, Serialize};

use self::client::execute_request;

#[derive(Debug, Deserialize)]
pub struct FetchRequestPayload {
    pub url: String,
    pub method: String,
    #[serde(default = "default_redirect_mode")]
    pub redirect_mode: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default)]
    pub headers: Vec<HeaderEntry>,
    #[serde(default)]
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct FetchResponsePayload {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub redirected: bool,
    pub is_redirect_response: bool,
    pub headers: Vec<HeaderEntry>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

fn default_redirect_mode() -> String {
    "follow".to_owned()
}

pub fn perform_fetch(payload: &str) -> Result<String, String> {
    let request: FetchRequestPayload =
        serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let response = execute_request(&request)?;

    serde_json::to_string(&response).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::perform_fetch;
    use rcgen::generate_simple_self_signed;
    use rustls::{
        ServerConfig, ServerConnection, StreamOwned,
        pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
    };
    use std::{
        io::{ErrorKind, Read, Write, copy},
        net::{Shutdown, TcpListener, TcpStream},
        sync::{Arc, Once},
        thread,
        time::Duration,
    };

    fn read_http_message(stream: &mut impl Read) -> String {
        let mut buffer = Vec::new();
        let mut chunk = [0_u8; 1024];

        loop {
            let read = stream.read(&mut chunk).unwrap();
            if read == 0 {
                break;
            }

            buffer.extend_from_slice(&chunk[..read]);
            if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        String::from_utf8_lossy(&buffer).into_owned()
    }

    fn install_rustls_provider() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
    }

    fn make_tls_server_config(hostname: &str) -> Arc<ServerConfig> {
        let certified = generate_simple_self_signed(vec![hostname.to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            certified.signing_key.serialize_der(),
        ));

        Arc::new(
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key)
                .unwrap(),
        )
    }

    fn relay_tls_tunnel(
        inbound: &mut StreamOwned<ServerConnection, TcpStream>,
        outbound: &mut TcpStream,
    ) {
        inbound
            .get_mut()
            .set_read_timeout(Some(Duration::from_millis(25)))
            .unwrap();
        outbound
            .set_read_timeout(Some(Duration::from_millis(25)))
            .unwrap();

        let mut inbound_open = true;
        let mut outbound_open = true;
        let mut inbound_buffer = [0_u8; 8192];
        let mut outbound_buffer = [0_u8; 8192];

        while inbound_open || outbound_open {
            let mut progress = false;

            if inbound_open {
                match inbound.read(&mut inbound_buffer) {
                    Ok(0) => {
                        inbound_open = false;
                        let _ = outbound.shutdown(Shutdown::Write);
                    }
                    Ok(read) => {
                        outbound.write_all(&inbound_buffer[..read]).unwrap();
                        outbound.flush().unwrap();
                        progress = true;
                    }
                    Err(error)
                        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(error)
                        if error.kind() == ErrorKind::UnexpectedEof
                            || error.to_string().contains("close_notify") =>
                    {
                        inbound_open = false;
                        let _ = outbound.shutdown(Shutdown::Write);
                    }
                    Err(error) => panic!("tls tunnel read failed: {error}"),
                }
            }

            if outbound_open {
                match outbound.read(&mut outbound_buffer) {
                    Ok(0) => {
                        outbound_open = false;
                        let _ = inbound.get_mut().shutdown(Shutdown::Write);
                    }
                    Ok(read) => {
                        inbound.write_all(&outbound_buffer[..read]).unwrap();
                        inbound.flush().unwrap();
                        progress = true;
                    }
                    Err(error)
                        if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(error) => panic!("tls tunnel writeback failed: {error}"),
                }
            }

            if !progress && !inbound_open && !outbound_open {
                break;
            }
        }
    }

    fn read_socks5_address(stream: &mut TcpStream, address_type: u8) -> String {
        match address_type {
            0x01 => {
                let mut address = [0_u8; 4];
                stream.read_exact(&mut address).unwrap();
                std::net::Ipv4Addr::from(address).to_string()
            }
            0x03 => {
                let mut len = [0_u8; 1];
                stream.read_exact(&mut len).unwrap();
                let mut domain = vec![0_u8; len[0] as usize];
                stream.read_exact(&mut domain).unwrap();
                String::from_utf8(domain).unwrap()
            }
            0x04 => {
                let mut address = [0_u8; 16];
                stream.read_exact(&mut address).unwrap();
                std::net::Ipv6Addr::from(address).to_string()
            }
            _ => panic!("unsupported socks5 address type"),
        }
    }

    fn spawn_socks5_proxy(
        credentials: Option<(&'static str, &'static str)>,
    ) -> (std::net::SocketAddr, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut inbound, _) = listener.accept().unwrap();

            let mut greeting_head = [0_u8; 2];
            inbound.read_exact(&mut greeting_head).unwrap();
            assert_eq!(greeting_head[0], 0x05);

            let mut methods = vec![0_u8; greeting_head[1] as usize];
            inbound.read_exact(&mut methods).unwrap();

            let selected_method = if credentials.is_some() { 0x02 } else { 0x00 };
            assert!(methods.contains(&selected_method));
            inbound.write_all(&[0x05, selected_method]).unwrap();
            inbound.flush().unwrap();

            if let Some((expected_username, expected_password)) = credentials {
                let mut auth_version = [0_u8; 1];
                inbound.read_exact(&mut auth_version).unwrap();
                assert_eq!(auth_version[0], 0x01);

                let mut username_len = [0_u8; 1];
                inbound.read_exact(&mut username_len).unwrap();
                let mut username = vec![0_u8; username_len[0] as usize];
                inbound.read_exact(&mut username).unwrap();

                let mut password_len = [0_u8; 1];
                inbound.read_exact(&mut password_len).unwrap();
                let mut password = vec![0_u8; password_len[0] as usize];
                inbound.read_exact(&mut password).unwrap();

                assert_eq!(String::from_utf8(username).unwrap(), expected_username);
                assert_eq!(String::from_utf8(password).unwrap(), expected_password);

                inbound.write_all(&[0x01, 0x00]).unwrap();
                inbound.flush().unwrap();
            }

            let mut request_head = [0_u8; 4];
            inbound.read_exact(&mut request_head).unwrap();
            assert_eq!(request_head[0], 0x05);
            assert_eq!(request_head[1], 0x01);

            let target_host = read_socks5_address(&mut inbound, request_head[3]);
            let mut port_bytes = [0_u8; 2];
            inbound.read_exact(&mut port_bytes).unwrap();
            let target_port = u16::from_be_bytes(port_bytes);

            let mut outbound = TcpStream::connect((target_host.as_str(), target_port)).unwrap();
            inbound
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .unwrap();
            inbound.flush().unwrap();

            let mut inbound_reader = inbound.try_clone().unwrap();
            let mut outbound_reader = outbound.try_clone().unwrap();

            let upstream = thread::spawn(move || {
                let _ = copy(&mut inbound_reader, &mut outbound);
                let _ = outbound.shutdown(Shutdown::Write);
            });

            let downstream = thread::spawn(move || {
                let _ = copy(&mut outbound_reader, &mut inbound);
            });

            upstream.join().unwrap();
            downstream.join().unwrap();
        });

        (address, server)
    }

    #[test]
    fn supports_https_fetch_through_http_connect_proxy() {
        install_rustls_provider();
        let server_config = make_tls_server_config("localhost");

        let https_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let https_address = https_listener.local_addr().unwrap();

        let https_server = thread::spawn(move || {
            let (stream, _) = https_listener.accept().unwrap();
            let connection = ServerConnection::new(server_config).unwrap();
            let mut tls_stream = StreamOwned::new(connection, stream);

            let request = read_http_message(&mut tls_stream);
            assert!(request.contains("GET /secure HTTP/1.1"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 5\r\n",
                "\r\n",
                "proxy"
            );

            tls_stream.write_all(response.as_bytes()).unwrap();
            tls_stream.flush().unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_address = proxy_listener.local_addr().unwrap();

        let proxy_server = thread::spawn(move || {
            let (mut inbound, _) = proxy_listener.accept().unwrap();
            let connect_request = read_http_message(&mut inbound);
            assert!(connect_request.contains(&format!(
                "CONNECT localhost:{} HTTP/1.1",
                https_address.port()
            )));
            assert!(connect_request.contains("Proxy-Authorization: Basic dXNlcjpwYXNz"));

            inbound
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
            inbound.flush().unwrap();

            let mut outbound = TcpStream::connect(https_address).unwrap();
            let mut inbound_reader = inbound.try_clone().unwrap();
            let mut outbound_reader = outbound.try_clone().unwrap();

            let upstream = thread::spawn(move || {
                let _ = copy(&mut inbound_reader, &mut outbound);
                let _ = outbound.shutdown(Shutdown::Write);
            });

            let downstream = thread::spawn(move || {
                let _ = copy(&mut outbound_reader, &mut inbound);
            });

            upstream.join().unwrap();
            downstream.join().unwrap();
        });

        let payload = serde_json::json!({
            "url": format!("https://localhost:{}/secure", https_address.port()),
            "method": "GET",
            "redirect_mode": "follow",
            "proxy_url": format!("http://user:pass@127.0.0.1:{}", proxy_address.port()),
            "headers": [],
            "body": [],
        });

        let response = perform_fetch(&payload.to_string()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        proxy_server.join().unwrap();
        https_server.join().unwrap();

        assert_eq!(value["status"], 200);
        assert_eq!(value["body"], serde_json::json!([112, 114, 111, 120, 121]));
    }

    #[test]
    fn supports_https_fetch_through_https_connect_proxy() {
        install_rustls_provider();
        let origin_config = make_tls_server_config("localhost");
        let proxy_config = make_tls_server_config("localhost");

        let https_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let https_address = https_listener.local_addr().unwrap();

        let https_server = thread::spawn(move || {
            let (stream, _) = https_listener.accept().unwrap();
            let connection = ServerConnection::new(origin_config).unwrap();
            let mut tls_stream = StreamOwned::new(connection, stream);

            let request = read_http_message(&mut tls_stream);
            assert!(request.contains("GET /secure HTTP/1.1"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 9\r\n",
                "\r\n",
                "httpsprox"
            );

            tls_stream.write_all(response.as_bytes()).unwrap();
            tls_stream.flush().unwrap();
        });

        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_address = proxy_listener.local_addr().unwrap();

        let proxy_server = thread::spawn(move || {
            let (stream, _) = proxy_listener.accept().unwrap();
            let connection = ServerConnection::new(proxy_config).unwrap();
            let mut tls_stream = StreamOwned::new(connection, stream);

            let connect_request = read_http_message(&mut tls_stream);
            assert!(connect_request.contains(&format!(
                "CONNECT localhost:{} HTTP/1.1",
                https_address.port()
            )));
            assert!(connect_request.contains("Proxy-Authorization: Basic dXNlcjpwYXNz"));

            tls_stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
            tls_stream.flush().unwrap();

            let mut outbound = TcpStream::connect(https_address).unwrap();
            relay_tls_tunnel(&mut tls_stream, &mut outbound);
        });

        let payload = serde_json::json!({
            "url": format!("https://localhost:{}/secure", https_address.port()),
            "method": "GET",
            "redirect_mode": "follow",
            "proxy_url": format!("https://user:pass@127.0.0.1:{}", proxy_address.port()),
            "headers": [],
            "body": [],
        });

        let response = perform_fetch(&payload.to_string()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        proxy_server.join().unwrap();
        https_server.join().unwrap();

        assert_eq!(value["status"], 200);
        assert_eq!(
            value["body"],
            serde_json::json!([104, 116, 116, 112, 115, 112, 114, 111, 120])
        );
    }

    #[test]
    fn supports_http_fetch_through_socks5_proxy() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_message(&mut stream);
            assert!(request.contains("GET /plain HTTP/1.1"));
            assert!(!request.contains("GET http://"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 5\r\n",
                "\r\n",
                "socks"
            );

            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        });

        let (proxy_address, proxy_server) = spawn_socks5_proxy(None);
        let payload = serde_json::json!({
            "url": format!("http://localhost:{}/plain", address.port()),
            "method": "GET",
            "redirect_mode": "follow",
            "proxy_url": format!("socks5://127.0.0.1:{}", proxy_address.port()),
            "headers": [],
            "body": [],
        });

        let response = perform_fetch(&payload.to_string()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        proxy_server.join().unwrap();
        server.join().unwrap();

        assert_eq!(value["status"], 200);
        assert_eq!(value["body"], serde_json::json!([115, 111, 99, 107, 115]));
    }

    #[test]
    fn supports_https_fetch_through_socks5_proxy_with_authentication() {
        install_rustls_provider();

        let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            certified.signing_key.serialize_der(),
        ));

        let server_config = Arc::new(
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certificate], private_key)
                .unwrap(),
        );

        let https_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let https_address = https_listener.local_addr().unwrap();

        let https_server = thread::spawn(move || {
            let (stream, _) = https_listener.accept().unwrap();
            let connection = ServerConnection::new(server_config).unwrap();
            let mut tls_stream = StreamOwned::new(connection, stream);

            let request = read_http_message(&mut tls_stream);
            assert!(request.contains("GET /secure HTTP/1.1"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 6\r\n",
                "\r\n",
                "tunnel"
            );

            tls_stream.write_all(response.as_bytes()).unwrap();
            tls_stream.flush().unwrap();
        });

        let (proxy_address, proxy_server) = spawn_socks5_proxy(Some(("user", "pass")));
        let payload = serde_json::json!({
            "url": format!("https://localhost:{}/secure", https_address.port()),
            "method": "GET",
            "redirect_mode": "follow",
            "proxy_url": format!("socks5://user:pass@127.0.0.1:{}", proxy_address.port()),
            "headers": [],
            "body": [],
        });

        let response = perform_fetch(&payload.to_string()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response).unwrap();

        proxy_server.join().unwrap();
        https_server.join().unwrap();

        assert_eq!(value["status"], 200);
        assert_eq!(
            value["body"],
            serde_json::json!([116, 117, 110, 110, 101, 108])
        );
    }
}
