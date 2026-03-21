use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut expected_total = None;

    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..read]);

        if header_end.is_none() {
            header_end = buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4);

            if let Some(end) = header_end {
                let headers = String::from_utf8_lossy(&buffer[..end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let lower = line.to_ascii_lowercase();
                        lower
                            .strip_prefix("content-length: ")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                expected_total = Some(end + content_length);
            }
        }

        if let Some(total) = expected_total
            && buffer.len() >= total
        {
            break;
        }
    }

    String::from_utf8_lossy(&buffer).into_owned()
}

#[test]
fn supports_cors_simple_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);

        assert!(request.contains("GET /simple HTTP/1.1"));
        assert!(request.contains("origin: https://rom.local"));

        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "Access-Control-Allow-Origin: https://rom.local\r\n",
            "X-Secret: hidden\r\n",
            "Content-Length: 11\r\n",
            "\r\n",
            "{\"ok\":true}"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        cors_enabled: true,
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = format!(
        r#"
        (async () => {{
            const response = await fetch("http://{address}/simple");
            return {{
                ok: response.ok,
                type: response.type,
                secret: response.headers.get("x-secret"),
                contentType: response.headers.get("content-type"),
                body: await response.json(),
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["type"], "cors");
    assert_eq!(value["secret"], serde_json::Value::Null);
    assert_eq!(value["contentType"], "application/json");
    assert_eq!(value["body"]["ok"], true);
}

#[test]
fn supports_cors_preflight_and_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for request_index in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);

            if request_index == 0 {
                assert!(request.contains("GET /seed HTTP/1.1"));
                assert!(request.contains("origin: https://rom.local"));
                assert!(!request.contains("cookie:"));

                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Access-Control-Allow-Origin: https://rom.local\r\n",
                    "Access-Control-Allow-Credentials: true\r\n",
                    "Set-Cookie: sid=abc; Path=/; SameSite=None; HttpOnly\r\n",
                    "Content-Length: 4\r\n",
                    "\r\n",
                    "seed"
                );

                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            if request_index == 1 {
                assert!(request.contains("OPTIONS /write HTTP/1.1"));
                assert!(request.contains("origin: https://rom.local"));
                assert!(request.contains("access-control-request-method: PUT"));
                assert!(
                    request.contains("access-control-request-headers: content-type, x-rom-test")
                );
                assert!(!request.contains("cookie:"));

                let response = concat!(
                    "HTTP/1.1 204 No Content\r\n",
                    "Access-Control-Allow-Origin: https://rom.local\r\n",
                    "Access-Control-Allow-Credentials: true\r\n",
                    "Access-Control-Allow-Methods: PUT\r\n",
                    "Access-Control-Allow-Headers: content-type, x-rom-test\r\n",
                    "Content-Length: 0\r\n",
                    "\r\n"
                );

                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            assert!(request.contains("PUT /write HTTP/1.1"));
            assert!(request.contains("origin: https://rom.local"));
            assert!(request.contains("cookie: sid=abc"));
            assert!(request.contains("x-rom-test: yes"));
            assert!(request.contains("{\"step\":2}"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "Access-Control-Allow-Origin: https://rom.local\r\n",
                "Access-Control-Allow-Credentials: true\r\n",
                "Access-Control-Expose-Headers: X-Reply\r\n",
                "X-Reply: ok\r\n",
                "Content-Length: 11\r\n",
                "\r\n",
                "{\"ok\":true}"
            );

            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        cors_enabled: true,
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = format!(
        r#"
        (async () => {{
            await fetch("http://{address}/seed", {{
                credentials: "include",
            }});

            const response = await fetch("http://{address}/write", {{
                method: "PUT",
                credentials: "include",
                headers: {{
                    "content-type": "application/json",
                    "x-rom-test": "yes",
                }},
                body: JSON.stringify({{ step: 2 }}),
            }});

            return {{
                type: response.type,
                reply: response.headers.get("x-reply"),
                documentCookie: document.cookie,
                payload: await response.json(),
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["type"], "cors");
    assert_eq!(value["reply"], "ok");
    assert_eq!(value["documentCookie"], "");
    assert_eq!(value["payload"]["ok"], true);
}

#[test]
fn rejects_cors_response_without_allow_origin() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);

        assert!(request.contains("GET /blocked HTTP/1.1"));
        assert!(request.contains("origin: https://rom.local"));

        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: text/plain\r\n",
            "Content-Length: 7\r\n",
            "\r\n",
            "blocked"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        cors_enabled: true,
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = format!(
        r#"
        (async () => {{
            try {{
                await fetch("http://{address}/blocked");
                return "unexpected";
            }} catch (error) {{
                return String(error.message ?? error);
            }}
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    assert_eq!(result, "Failed to fetch");
}

#[test]
fn does_not_send_or_store_cross_origin_cookies_with_default_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);

        assert!(request.contains("GET /default-creds HTTP/1.1"));
        assert!(request.contains("origin: https://rom.local"));
        assert!(!request.contains("cookie:"));

        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: text/plain\r\n",
            "Access-Control-Allow-Origin: https://rom.local\r\n",
            "Set-Cookie: sid=abc; Path=/; SameSite=None\r\n",
            "Content-Length: 2\r\n",
            "\r\n",
            "ok"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        cors_enabled: true,
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = format!(
        r#"
        (async () => {{
            const response = await fetch("http://{address}/default-creds");
            return {{
                text: await response.text(),
                documentCookie: document.cookie,
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["text"], "ok");
    assert_eq!(value["documentCookie"], "");
}

#[test]
fn disables_cors_enforcement_by_default() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);

        assert!(request.contains("GET /open HTTP/1.1"));
        assert!(!request.contains("origin: https://rom.local"));

        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "X-Secret: visible\r\n",
            "Content-Length: 11\r\n",
            "\r\n",
            "{\"ok\":true}"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const response = await fetch("http://{address}/open");
            return {{
                ok: response.ok,
                type: response.type,
                secret: response.headers.get("x-secret"),
                body: await response.json(),
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["type"], "basic");
    assert_eq!(value["secret"], "visible");
    assert_eq!(value["body"]["ok"], true);
}
