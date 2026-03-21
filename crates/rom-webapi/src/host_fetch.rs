mod client;
mod proxy;

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
        io::{Read, Write, copy},
        net::{Shutdown, TcpListener, TcpStream},
        sync::{Arc, Once},
        thread,
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

    #[test]
    fn supports_https_fetch_through_http_connect_proxy() {
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
}
