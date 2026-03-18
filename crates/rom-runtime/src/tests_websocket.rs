use crate::{RomRuntime, RuntimeConfig};
use std::{
    net::TcpListener,
    sync::mpsc,
    thread,
    time::Duration,
};
use tungstenite::{Message, accept};

#[test]
fn supports_websocket_text_binary_and_close_events() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();
        websocket.send(Message::Text("welcome".into())).unwrap();

        let message = websocket.read().unwrap();
        match message {
            Message::Binary(bytes) => {
                assert_eq!(bytes.as_ref(), &[1, 2, 3]);
                websocket.send(Message::Binary(bytes)).unwrap();
            }
            other => panic!("unexpected websocket message: {other:?}"),
        }

        websocket.close(None).unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("ws://{address}/socket");
            socket.binaryType = "arraybuffer";
            const events = [];

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => {{
                    events.push({{ type: "open", readyState: socket.readyState }});
                    socket.send(new Uint8Array([1, 2, 3]));
                }};

                socket.onmessage = (event) => {{
                    if (typeof event.data === "string") {{
                        events.push({{
                            type: "text",
                            data: event.data,
                        }});
                        return;
                    }}

                    events.push({{
                        type: "binary",
                        bytes: Array.from(new Uint8Array(event.data)),
                    }});
                }};

                socket.onclose = (event) => {{
                    resolve({{
                        events,
                        closeCode: event.code,
                        closeReason: event.reason,
                        wasClean: event.wasClean,
                        isCloseEvent: event instanceof CloseEvent,
                        finalReadyState: socket.readyState,
                    }});
                }};

                socket.onerror = () => reject(new Error("unexpected websocket error"));
            }});
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["events"],
        serde_json::json!([
            { "type": "open", "readyState": 1 },
            { "type": "text", "data": "welcome" },
            { "type": "binary", "bytes": [1, 2, 3] }
        ])
    );
    assert_eq!(value["closeCode"], 1000);
    assert_eq!(value["closeReason"], "");
    assert_eq!(value["wasClean"], true);
    assert_eq!(value["isCloseEvent"], true);
    assert_eq!(value["finalReadyState"], 3);
}

#[test]
fn supports_websocket_blob_payloads_and_default_blob_binary_type() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();

        let message = websocket.read().unwrap();
        match message {
            Message::Binary(bytes) => {
                assert_eq!(bytes.as_ref(), b"blob-payload");
                websocket.send(Message::Binary(bytes)).unwrap();
            }
            other => panic!("unexpected websocket message: {other:?}"),
        }

        websocket.close(None).unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("ws://{address}/blob");

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => {{
                    socket.send(new Blob(["blob-", new Uint8Array([112, 97, 121, 108, 111, 97, 100])]));
                }};

                socket.onmessage = async (event) => {{
                    resolve({{
                        isBlob: event.data instanceof Blob,
                        echoedText: await event.data.text(),
                        size: event.data.size,
                    }});
                }};

                socket.onerror = () => reject(new Error("unexpected websocket error"));
            }});
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["isBlob"], true);
    assert_eq!(value["echoedText"], "blob-payload");
    assert_eq!(value["size"], 12);
}

#[test]
fn supports_websocket_messages_after_idle_delay() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();

        thread::sleep(Duration::from_millis(75));
        websocket.send(Message::Text("late-message".into())).unwrap();
        websocket.close(None).unwrap();
    });

    let config = RuntimeConfig::default();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("ws://{address}/delayed");

            return await new Promise((resolve, reject) => {{
                const events = [];

                socket.onopen = () => {{
                    events.push("open");
                }};

                socket.onmessage = (event) => {{
                    events.push(event.data);
                }};

                socket.onclose = () => {{
                    resolve(events);
                }};

                socket.onerror = () => {{
                    reject(new Error("unexpected websocket error"));
                }};
            }});
        }})()
        "#
    );

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let runtime = RomRuntime::new(config).unwrap();
        let _ = sender.send(runtime.eval_async_as_string(&script));
    });

    let result = receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("timed out waiting for websocket script result")
        .unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value, serde_json::json!(["open", "late-message"]));
}
