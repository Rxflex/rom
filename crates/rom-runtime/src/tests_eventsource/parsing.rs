use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn supports_utf8_text_encoding_and_decoding() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = r#"
        (() => JSON.stringify({
            encoded: Array.from(new TextEncoder().encode("hé🙂")),
            decoded: new TextDecoder().decode(Uint8Array.from([104, 195, 169, 240, 159, 153, 130])),
            bomDecoded: new TextDecoder().decode(Uint8Array.from([239, 187, 191, 104, 105])),
        }))()
    "#;

    let result = runtime.eval_as_string(script).unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["encoded"], serde_json::json!([104, 195, 169, 240, 159, 153, 130]));
    assert_eq!(value["decoded"], "hé🙂");
    assert_eq!(value["bomDecoded"], "hi");
}

#[test]
fn ignores_leading_bom_in_eventsource_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]);

        assert!(request.contains("GET /events HTTP/1.1"));

        let body = "\u{FEFF}data: hello\n\n";
        let response = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "Cache-Control: no-cache\r\n",
                "Content-Length: {}\r\n",
                "\r\n",
                "{}"
            ),
            body.len(),
            body,
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        href: format!("http://{address}/"),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = r#"
        (async () => {
            const source = new EventSource("/events");

            return await new Promise((resolve) => {
                const events = [];

                source.onmessage = (event) => {
                    source.close();
                    events.push({
                        data: event.data,
                        type: event.type,
                    });
                    resolve(JSON.stringify({ events }));
                };

                source.onerror = () => {
                    events.push({
                        type: "error",
                        readyState: source.readyState,
                    });
                    setTimeout(() => resolve(JSON.stringify({ events })), 25);
                };
            });
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["events"][0]["data"], "hello");
    assert_eq!(value["events"][0]["type"], "message");
}

#[test]
fn supports_carriage_return_delimited_eventsource_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]);

        assert!(request.contains("GET /events HTTP/1.1"));

        let body = "event: custom\rdata: alpha\rdata: beta\r\r";
        let response = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "Cache-Control: no-cache\r\n",
                "Content-Length: {}\r\n",
                "\r\n",
                "{}"
            ),
            body.len(),
            body,
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        href: format!("http://{address}/"),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = r#"
        (async () => {
            const source = new EventSource("/events");

            return await new Promise((resolve, reject) => {
                source.addEventListener("custom", (event) => {
                    source.close();
                    resolve(JSON.stringify({
                        data: event.data,
                        type: event.type,
                    }));
                });

                source.onerror = () => reject(new Error("unexpected EventSource error"));
            });
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["data"], "alpha\nbeta");
    assert_eq!(value["type"], "custom");
}
