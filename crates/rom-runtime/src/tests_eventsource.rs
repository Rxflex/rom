use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
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

#[test]
fn supports_eventsource_reconnect_retry_and_last_event_id() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut first_stream, _) = listener.accept().unwrap();
        let mut first_buffer = [0_u8; 2048];
        let first_read = first_stream.read(&mut first_buffer).unwrap();
        let first_request = String::from_utf8_lossy(&first_buffer[..first_read]);

        assert!(first_request.contains("GET /events HTTP/1.1"));
        assert!(first_request.contains("accept: text/event-stream"));
        assert!(!first_request.contains("last-event-id:"));

        let first_body = concat!("retry: 25\n", "id: 1\n", "data: first\n", "\n");
        let first_response = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "Cache-Control: no-cache\r\n",
                "Content-Length: {}\r\n",
                "\r\n",
                "{}"
            ),
            first_body.len(),
            first_body,
        );
        first_stream.write_all(first_response.as_bytes()).unwrap();
        first_stream.flush().unwrap();

        let (mut second_stream, _) = listener.accept().unwrap();
        let mut second_buffer = [0_u8; 2048];
        let second_read = second_stream.read(&mut second_buffer).unwrap();
        let second_request = String::from_utf8_lossy(&second_buffer[..second_read]);

        assert!(second_request.contains("GET /events HTTP/1.1"));
        assert!(second_request.contains("last-event-id: 1"));

        let second_body = concat!("data: second\n", "\n");
        let second_response = format!(
            concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/event-stream\r\n",
                "Cache-Control: no-cache\r\n",
                "Content-Length: {}\r\n",
                "\r\n",
                "{}"
            ),
            second_body.len(),
            second_body,
        );
        second_stream.write_all(second_response.as_bytes()).unwrap();
        second_stream.flush().unwrap();
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
                source.onopen = () => {
                    events.push({ type: "open", readyState: source.readyState });
                };

                source.onmessage = (event) => {
                    events.push({
                        type: event.type,
                        data: event.data,
                        lastEventId: event.lastEventId,
                    });

                    if (event.data === "second") {
                        source.close();
                        resolve({
                            events,
                            readyState: source.readyState,
                        });
                    }
                };

                source.onerror = () => {
                    events.push({ type: "error", readyState: source.readyState });
                };
            });
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["events"],
        serde_json::json!([
            { "type": "open", "readyState": 1 },
            { "type": "message", "data": "first", "lastEventId": "1" },
            { "type": "error", "readyState": 0 },
            { "type": "open", "readyState": 1 },
            { "type": "message", "data": "second", "lastEventId": "1" }
        ])
    );
    assert_eq!(value["readyState"], 2);
}

#[test]
fn closes_eventsource_on_fatal_http_failure_without_reconnect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(200);
        let mut accepted = 0usize;

        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    accepted += 1;
                    let mut buffer = [0_u8; 2048];
                    let read = stream.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..read]);

                    assert!(request.contains("GET /events HTTP/1.1"));

                    let response = concat!(
                        "HTTP/1.1 500 Internal Server Error\r\n",
                        "Content-Type: text/plain\r\n",
                        "Content-Length: 0\r\n",
                        "\r\n"
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    stream.flush().unwrap();
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("unexpected accept error: {error}"),
            }
        }

        accepted
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

            return await new Promise((resolve) => {
                source.onerror = () => {
                    events.push({ type: "error", readyState: source.readyState });
                    setTimeout(() => {
                        resolve({
                            events,
                            readyState: source.readyState,
                            url: source.url,
                        });
                    }, 80);
                };
            });
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();
    let accepted = server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(accepted, 1);
    assert_eq!(
        value["events"],
        serde_json::json!([{ "type": "error", "readyState": 2 }])
    );
    assert_eq!(value["readyState"], 2);
    assert_eq!(value["url"], format!("http://{address}/events"));
}
