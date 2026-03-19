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
        (() => {
            const fullBuffer = new Uint8Array(8);
            const fullEncodeInto = new TextEncoder().encodeInto("hé🙂", fullBuffer);
            const partialBuffer = new Uint8Array(4);
            const partialEncodeInto = new TextEncoder().encodeInto("A🙂B", partialBuffer);

            return JSON.stringify({
                encoded: Array.from(new TextEncoder().encode("hé🙂")),
                decoded: new TextDecoder().decode(Uint8Array.from([104, 195, 169, 240, 159, 153, 130])),
                bomDecoded: new TextDecoder().decode(Uint8Array.from([239, 187, 191, 104, 105])),
                bomIncluded: new TextDecoder(" utf8 ", { ignoreBOM: true }).decode(
                    Uint8Array.from([239, 187, 191, 104, 105])
                ),
                nullOptionsEncoding: new TextDecoder(" UTF-8 ", null).encoding,
                nullOptionsFatal: new TextDecoder(" UTF-8 ", null).fatal,
                aliasDecoded: new TextDecoder(" unicode-1-1-utf-8 ").decode(
                    Uint8Array.from([104, 195, 169])
                ),
                legacyAliasDecoded: new TextDecoder("x-unicode20utf8").decode(
                    Uint8Array.from([104, 195, 169])
                ),
                truncatedReplacement: new TextDecoder().decode(Uint8Array.from([240, 159, 153])),
                continuationReplacement: new TextDecoder().decode(
                    Uint8Array.from([240, 159, 153, 65])
                ),
                fatalErrorName: (() => {
                    try {
                        new TextDecoder("utf-8", { fatal: true }).decode(
                            Uint8Array.from([240, 159, 153])
                        );
                        return "ok";
                    } catch (error) {
                        return String(error.name);
                    }
                })(),
                fullEncodeInto,
                fullBuffer: Array.from(fullBuffer),
                partialEncodeInto,
                partialBuffer: Array.from(partialBuffer),
            });
        })()
    "#;

    let result = runtime.eval_as_string(script).unwrap();
    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["encoded"], serde_json::json!([104, 195, 169, 240, 159, 153, 130]));
    assert_eq!(value["decoded"], "hé🙂");
    assert_eq!(value["bomDecoded"], "hi");
    assert_eq!(value["bomIncluded"], "\u{feff}hi");
    assert_eq!(value["nullOptionsEncoding"], "utf-8");
    assert_eq!(value["nullOptionsFatal"], false);
    assert_eq!(value["aliasDecoded"], "hé");
    assert_eq!(value["legacyAliasDecoded"], "hé");
    assert_eq!(value["truncatedReplacement"], "\u{fffd}");
    assert_eq!(value["continuationReplacement"], "\u{fffd}A");
    assert_eq!(value["fatalErrorName"], "TypeError");
    assert_eq!(value["fullEncodeInto"], serde_json::json!({ "read": 4, "written": 7 }));
    assert_eq!(
        value["fullBuffer"],
        serde_json::json!([104, 195, 169, 240, 159, 153, 130, 0])
    );
    assert_eq!(value["partialEncodeInto"], serde_json::json!({ "read": 1, "written": 1 }));
    assert_eq!(value["partialBuffer"], serde_json::json!([65, 0, 0, 0]));
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
