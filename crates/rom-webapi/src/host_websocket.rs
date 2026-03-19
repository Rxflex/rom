use std::{
    collections::HashMap,
    net::TcpStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::{Deserialize, Serialize};
use tungstenite::{
    Connector, Message, WebSocket,
    client::IntoClientRequest,
    client_tls_with_config,
    handshake::client::Response,
    protocol::{CloseFrame, frame::Utf8Bytes, frame::coding::CloseCode},
    stream::MaybeTlsStream,
};
use url::Url;

#[derive(Clone, Default)]
pub struct WebSocketHost {
    next_socket_id: Arc<AtomicU64>,
    sockets: Arc<Mutex<HashMap<String, WebSocketSession>>>,
}

struct WebSocketSession {
    socket: WebSocket<MaybeTlsStream<TcpStream>>,
    closed: bool,
    close_code: u16,
    close_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketConnectPayload {
    pub url: String,
    #[serde(default)]
    pub protocols: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketSendPayload {
    pub socket_id: String,
    pub kind: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketPollPayload {
    pub socket_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketClosePayload {
    pub socket_id: String,
    #[serde(default)]
    pub code: Option<u16>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct WebSocketConnectResult {
    socket_id: String,
    url: String,
    protocol: String,
}

#[derive(Debug, Serialize)]
struct WebSocketPollResult {
    messages: Vec<WebSocketFrame>,
    close_event: Option<WebSocketCloseEvent>,
}

#[derive(Debug, Serialize)]
struct WebSocketFrame {
    kind: &'static str,
    #[serde(skip_serializing_if = "String::is_empty")]
    text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct WebSocketCloseEvent {
    code: u16,
    reason: String,
    was_clean: bool,
}

impl WebSocketHost {
    pub fn connect(&self, payload: &str) -> Result<String, String> {
        let payload: WebSocketConnectPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let request = build_request(&payload)?;
        let (mut socket, response) = connect_socket(request, connect_stream(&payload.url)?, None)?;
        set_nonblocking(socket.get_mut())?;

        let socket_id = format!(
            "socket-{}",
            self.next_socket_id.fetch_add(1, Ordering::Relaxed)
        );
        let protocol = extract_protocol(&response);
        self.sockets
            .lock()
            .map_err(|_| "websocket store poisoned".to_owned())?
            .insert(
                socket_id.clone(),
                WebSocketSession {
                    socket,
                    closed: false,
                    close_code: 1000,
                    close_reason: String::new(),
                },
            );

        serde_json::to_string(&WebSocketConnectResult {
            socket_id,
            url: payload.url,
            protocol,
        })
        .map_err(|error| error.to_string())
    }

    pub fn send(&self, payload: &str) -> Result<(), String> {
        let payload: WebSocketSendPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let mut sockets = self
            .sockets
            .lock()
            .map_err(|_| "websocket store poisoned".to_owned())?;
        let session = sockets
            .get_mut(&payload.socket_id)
            .ok_or_else(|| format!("Unknown WebSocket id: {}", payload.socket_id))?;

        if session.closed {
            return Err("WebSocket is closed".to_owned());
        }

        let message = match payload.kind.as_str() {
            "text" => Message::Text(payload.text.into()),
            "binary" => Message::Binary(payload.bytes.into()),
            other => return Err(format!("Unsupported WebSocket payload kind: {other}")),
        };

        session
            .socket
            .send(message)
            .map_err(|error| error.to_string())
    }

    pub fn poll(&self, payload: &str) -> Result<String, String> {
        let payload: WebSocketPollPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let mut sockets = self
            .sockets
            .lock()
            .map_err(|_| "websocket store poisoned".to_owned())?;
        let session = sockets
            .get_mut(&payload.socket_id)
            .ok_or_else(|| format!("Unknown WebSocket id: {}", payload.socket_id))?;

        let mut messages = Vec::new();
        let close_event = loop {
            match session.socket.read() {
                Ok(Message::Text(text)) => messages.push(WebSocketFrame {
                    kind: "text",
                    text: text.to_string(),
                    bytes: Vec::new(),
                }),
                Ok(Message::Binary(bytes)) => messages.push(WebSocketFrame {
                    kind: "binary",
                    text: String::new(),
                    bytes: bytes.to_vec(),
                }),
                Ok(Message::Close(frame)) => {
                    let (code, reason) = frame
                        .map(|frame| (u16::from(frame.code), frame.reason.to_string()))
                        .unwrap_or((1000, String::new()));
                    session.closed = true;
                    session.close_code = code;
                    session.close_reason = reason.clone();
                    break Some(WebSocketCloseEvent {
                        code,
                        reason,
                        was_clean: true,
                    });
                }
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => continue,
                Err(tungstenite::Error::Io(error))
                    if error.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    break close_event_for(session);
                }
                Err(error) => {
                    session.closed = true;
                    session.close_code = 1006;
                    session.close_reason = error.to_string();
                    break Some(WebSocketCloseEvent {
                        code: 1006,
                        reason: session.close_reason.clone(),
                        was_clean: false,
                    });
                }
            }
        };

        serde_json::to_string(&WebSocketPollResult {
            messages,
            close_event,
        })
        .map_err(|error| error.to_string())
    }

    pub fn close(&self, payload: &str) -> Result<String, String> {
        let payload: WebSocketClosePayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let mut sockets = self
            .sockets
            .lock()
            .map_err(|_| "websocket store poisoned".to_owned())?;
        let mut session = sockets
            .remove(&payload.socket_id)
            .ok_or_else(|| format!("Unknown WebSocket id: {}", payload.socket_id))?;
        let code = payload.code.unwrap_or(1000);
        let reason = payload.reason.unwrap_or_default();

        let frame = CloseFrame {
            code: CloseCode::from(code),
            reason: Utf8Bytes::from(reason.clone()),
        };
        let _ = session.socket.close(Some(frame));
        session.closed = true;
        session.close_code = code;
        session.close_reason = reason.clone();

        serde_json::to_string(&WebSocketCloseEvent {
            code,
            reason,
            was_clean: true,
        })
        .map_err(|error| error.to_string())
    }
}

fn build_request(
    payload: &WebSocketConnectPayload,
) -> Result<tungstenite::http::Request<()>, String> {
    let mut request = payload
        .url
        .as_str()
        .into_client_request()
        .map_err(|error| error.to_string())?;

    if !payload.protocols.is_empty() {
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            payload.protocols.join(", ").parse().map_err(
                |error: tungstenite::http::header::InvalidHeaderValue| error.to_string(),
            )?,
        );
    }

    Ok(request)
}

fn connect_stream(url: &str) -> Result<TcpStream, String> {
    let parsed = Url::parse(url).map_err(|error| error.to_string())?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "WebSocket URL must include a host".to_owned())?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "WebSocket URL must include a known port".to_owned())?;
    TcpStream::connect((host, port)).map_err(|error| error.to_string())
}

fn connect_socket(
    request: tungstenite::http::Request<()>,
    stream: TcpStream,
    connector: Option<Connector>,
) -> Result<(WebSocket<MaybeTlsStream<TcpStream>>, Response), String> {
    client_tls_with_config(request, stream, None, connector).map_err(|error| error.to_string())
}

fn set_nonblocking(stream: &mut MaybeTlsStream<TcpStream>) -> Result<(), String> {
    match stream {
        MaybeTlsStream::Plain(stream) => stream
            .set_nonblocking(true)
            .map_err(|error| error.to_string()),
        MaybeTlsStream::Rustls(stream) => stream
            .sock
            .set_nonblocking(true)
            .map_err(|error| error.to_string()),
        _ => Err("Unsupported WebSocket transport stream".to_owned()),
    }
}

fn close_event_for(session: &WebSocketSession) -> Option<WebSocketCloseEvent> {
    if !session.closed {
        return None;
    }

    Some(WebSocketCloseEvent {
        code: session.close_code,
        reason: session.close_reason.clone(),
        was_clean: session.close_code != 1006,
    })
}

fn extract_protocol(response: &Response) -> String {
    response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        Connector, WebSocketConnectPayload, build_request, connect_socket, connect_stream,
        set_nonblocking,
    };
    use rcgen::generate_simple_self_signed;
    use rustls::{
        ClientConfig, RootCertStore, ServerConfig, ServerConnection, StreamOwned,
        pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
    };
    use std::{io::ErrorKind, net::TcpListener, sync::Arc, sync::Once, thread};
    use tungstenite::{Message, accept};

    fn install_rustls_provider() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
    }

    #[test]
    fn supports_wss_connections_with_rustls_streams() {
        install_rustls_provider();

        let certified = generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            certified.signing_key.serialize_der(),
        ));

        let server_config = Arc::new(
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], private_key)
                .unwrap(),
        );

        let mut roots = RootCertStore::empty();
        roots.add(certificate).unwrap();
        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        );

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let connection = ServerConnection::new(server_config).unwrap();
            let tls_stream = StreamOwned::new(connection, stream);
            let mut websocket = accept(tls_stream).unwrap();

            websocket.send(Message::Text("secure".into())).unwrap();
            websocket.close(None).unwrap();
        });

        let request = build_request(&WebSocketConnectPayload {
            url: format!("wss://localhost:{}/socket", address.port()),
            protocols: Vec::new(),
        })
        .unwrap();
        let stream = connect_stream(&format!("wss://localhost:{}/socket", address.port())).unwrap();
        let (mut socket, _) =
            connect_socket(request, stream, Some(Connector::Rustls(client_config))).unwrap();

        set_nonblocking(socket.get_mut()).unwrap();

        let message = loop {
            match socket.read() {
                Ok(Message::Text(text)) => break text.to_string(),
                Err(tungstenite::Error::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                    thread::yield_now();
                }
                other => panic!("unexpected websocket read result: {other:?}"),
            }
        };

        server.join().unwrap();
        assert_eq!(message, "secure");
    }
}
