use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn supports_eventsource_stream_events() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0_u8; 2048];
        let read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]);

        assert!(request.contains("GET /events HTTP/1.1"));
        assert!(request.contains("accept: text/event-stream"));

        let body = concat!(
            "id: 1\n",
            "event: custom\n",
            "data: alpha\n",
            "data: beta\n",
            "\n",
            "data: gamma\n",
            "\n"
        );
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
            const events = [];
            const source = new EventSource("/events");

            return await new Promise((resolve, reject) => {
                const finish = () => {
                    if (events.length < 3) {
                        return;
                    }

                    source.close();
                    resolve({
                        events,
                        readyState: source.readyState,
                        url: source.url,
                        withCredentials: source.withCredentials,
                    });
                };

                source.onopen = () => {
                    events.push({ type: "open", readyState: source.readyState });
                    finish();
                };

                source.addEventListener("custom", (event) => {
                    events.push({
                        type: event.type,
                        data: event.data,
                        lastEventId: event.lastEventId,
                        origin: event.origin,
                    });
                    finish();
                });

                source.onmessage = (event) => {
                    events.push({
                        type: event.type,
                        data: event.data,
                        lastEventId: event.lastEventId,
                        origin: event.origin,
                    });
                    finish();
                };

                source.onerror = () => reject(new Error("unexpected EventSource error"));
            });
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let events = value["events"].as_array().unwrap();

    assert_eq!(events[0]["type"], "open");
    assert_eq!(events[0]["readyState"], 1);
    assert_eq!(events[1]["type"], "custom");
    assert_eq!(events[1]["data"], "alpha\nbeta");
    assert_eq!(events[1]["lastEventId"], "1");
    assert_eq!(events[2]["type"], "message");
    assert_eq!(events[2]["data"], "gamma");
    assert_eq!(events[2]["lastEventId"], "1");
    assert_eq!(events[2]["origin"], format!("http://{address}"));
    assert_eq!(value["readyState"], 2);
    assert_eq!(value["url"], format!("http://{address}/events"));
    assert_eq!(value["withCredentials"], false);
}
