use crate::{RomRuntime, RuntimeConfig};
use std::{
    net::{Shutdown, TcpListener},
    sync::mpsc,
    thread,
    time::Duration,
};
use tungstenite::{
    Message, accept, accept_hdr,
    handshake::server::{Request, Response},
    http::HeaderValue,
};

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
        websocket
            .send(Message::Text("late-message".into()))
            .unwrap();
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

#[test]
fn supports_websocket_http_url_normalization() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();
        let message = websocket.read().unwrap();

        match message {
            Message::Close(frame) => {
                let frame = frame.expect("expected close frame");
                assert_eq!(u16::from(frame.code), 1000);
            }
            other => panic!("unexpected websocket message: {other:?}"),
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("http://127.0.0.1:{}/normalized");

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => socket.close();
                socket.onclose = () => resolve(socket.url);
                socket.onerror = () => reject(new Error("unexpected websocket error"));
            }});
        }})()
        "#,
        address.port()
    );

    let result = runtime.eval_async_as_string(&script).unwrap();

    server.join().unwrap();
    assert_eq!(
        result,
        format!("ws://127.0.0.1:{}/normalized", address.port())
    );
}

#[test]
fn validates_websocket_constructor_and_close_arguments() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();
        let message = websocket.read().unwrap();

        match message {
            Message::Close(frame) => {
                let frame = frame.expect("expected close frame");
                assert_eq!(u16::from(frame.code), 1000);
                assert_eq!(frame.reason.to_string(), "done");
            }
            other => panic!("unexpected websocket message: {other:?}"),
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const constructorErrors = {{}};

            try {{
                new WebSocket("ws://example.com/path#fragment");
            }} catch (error) {{
                constructorErrors.fragment = error.name;
            }}

            try {{
                new WebSocket("ftp://example.com/socket");
            }} catch (error) {{
                constructorErrors.scheme = error.name;
            }}

            try {{
                new WebSocket("ws://example.com/socket", ["chat", "chat"]);
            }} catch (error) {{
                constructorErrors.duplicateProtocol = error.name;
            }}

            try {{
                new WebSocket("ws://example.com/socket", ["bad protocol"]);
            }} catch (error) {{
                constructorErrors.invalidProtocol = error.name;
            }}

            const socket = new WebSocket("ws://{address}/close");
            const closeErrors = {{}};

            try {{
                socket.close(2000);
            }} catch (error) {{
                closeErrors.invalidCode = error.name;
            }}

            try {{
                socket.close(1000, "x".repeat(124));
            }} catch (error) {{
                closeErrors.longReason = error.name;
            }}

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => {{
                    closeErrors.readyStateBeforeValidClose = socket.readyState;
                    socket.close(1000, "done");
                }};

                socket.onclose = (event) => {{
                    resolve({{
                        constructorErrors,
                        closeErrors,
                        closeCode: event.code,
                        closeReason: event.reason,
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
    assert_eq!(value["constructorErrors"]["fragment"], "SyntaxError");
    assert_eq!(value["constructorErrors"]["scheme"], "SyntaxError");
    assert_eq!(
        value["constructorErrors"]["duplicateProtocol"],
        "SyntaxError"
    );
    assert_eq!(value["constructorErrors"]["invalidProtocol"], "SyntaxError");
    assert_eq!(value["closeErrors"]["invalidCode"], "InvalidAccessError");
    assert_eq!(value["closeErrors"]["longReason"], "SyntaxError");
    assert_eq!(value["closeErrors"]["readyStateBeforeValidClose"], 1);
    assert_eq!(value["closeCode"], 1000);
    assert_eq!(value["closeReason"], "done");
    assert_eq!(value["finalReadyState"], 3);
}

#[test]
fn validates_websocket_binary_type_values() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();
        websocket
            .send(Message::Binary(b"abc".to_vec().into()))
            .unwrap();
        websocket.close(None).unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("ws://{address}/binary-type");
            const result = {{
                defaultType: socket.binaryType,
                invalidName: "",
                typeAfterInvalid: "",
                finalType: "",
                isBlob: false,
                text: "",
            }};

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => {{
                    try {{
                        socket.binaryType = "bytes";
                    }} catch (error) {{
                        result.invalidName = error.name;
                    }}

                    result.typeAfterInvalid = socket.binaryType;
                    socket.binaryType = "blob";
                    result.finalType = socket.binaryType;
                }};

                socket.onmessage = async (event) => {{
                    result.isBlob = event.data instanceof Blob;
                    result.text = await event.data.text();
                }};

                socket.onclose = () => resolve(result);
                socket.onerror = () => reject(new Error("unexpected websocket error"));
            }});
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["defaultType"], "blob");
    assert_eq!(value["invalidName"], "TypeError");
    assert_eq!(value["typeAfterInvalid"], "blob");
    assert_eq!(value["finalType"], "blob");
    assert_eq!(value["isBlob"], true);
    assert_eq!(value["text"], "abc");
}

#[test]
fn dispatches_websocket_error_before_abnormal_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut websocket = accept(stream).unwrap();

        thread::sleep(Duration::from_millis(50));
        websocket.get_mut().shutdown(Shutdown::Both).unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const socket = new WebSocket("ws://{address}/abrupt");
            const events = [];

            return await new Promise((resolve, reject) => {{
                socket.onopen = () => {{
                    events.push("open");
                }};

                socket.onerror = () => {{
                    events.push("error");
                }};

                socket.onclose = (event) => {{
                    events.push("close");
                    resolve({{
                        events,
                        code: event.code,
                        wasClean: event.wasClean,
                        readyState: socket.readyState,
                    }});
                }};

                socket.onmessage = () => reject(new Error("unexpected websocket message"));
            }});
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["events"],
        serde_json::json!(["open", "error", "close"])
    );
    assert_eq!(value["code"], 1006);
    assert_eq!(value["wasClean"], false);
    assert_eq!(value["readyState"], 3);
}

#[test]
#[allow(clippy::result_large_err)]
fn validates_websocket_protocol_negotiation_and_argument_types() {
    let valid_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let valid_address = valid_listener.local_addr().unwrap();
    let invalid_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let invalid_address = invalid_listener.local_addr().unwrap();

    let valid_server = thread::spawn(move || {
        let (stream, _) = valid_listener.accept().unwrap();
        let callback = |request: &Request, mut response: Response| {
            let requested = request
                .headers()
                .get("Sec-WebSocket-Protocol")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            assert_eq!(requested, "chat, superchat");
            response.headers_mut().insert(
                "Sec-WebSocket-Protocol",
                HeaderValue::from_static("superchat"),
            );
            Ok(response)
        };
        let mut websocket = accept_hdr(stream, callback).unwrap();
        websocket.close(None).unwrap();
    });

    let invalid_server = thread::spawn(move || {
        let (stream, _) = invalid_listener.accept().unwrap();
        let callback = |_: &Request, mut response: Response| {
            response
                .headers_mut()
                .insert("Sec-WebSocket-Protocol", HeaderValue::from_static("bogus"));
            Ok(response)
        };
        let mut websocket = accept_hdr(stream, callback).unwrap();
        websocket.close(None).unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (() => {{
            const result = {{}};

            const negotiated = new WebSocket("ws://{valid_address}/protocol", ["chat", "superchat"]);
            result.negotiatedProtocol = negotiated.protocol;
            negotiated.close();

            try {{
                new WebSocket("ws://{invalid_address}/protocol", ["chat"]);
            }} catch (error) {{
                result.invalidNegotiation = error.name;
            }}

            try {{
                new WebSocket("ws://example.com/socket", 1);
            }} catch (error) {{
                result.invalidProtocolsType = error.name;
            }}

            return result;
        }})()
        "#
    );

    let result = runtime.eval_as_string(&script).unwrap();

    valid_server.join().unwrap();
    invalid_server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["negotiatedProtocol"], "superchat");
    assert_eq!(value["invalidNegotiation"], "TypeError");
    assert_eq!(value["invalidProtocolsType"], "TypeError");
}
