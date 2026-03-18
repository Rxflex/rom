use crate::{RomRuntime, RuntimeConfig};
use std::{net::TcpListener, thread};
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
