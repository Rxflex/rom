use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn supports_document_cookie_and_location_updates() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.cookie = "alpha=1; path=/";
                document.cookie = "beta=2; path=/admin";
                document.cookie = "gamma=3; secure; path=/";
                document.cookie = "foreign=9; domain=example.com; path=/";
                document.cookie = "alpha=gone; path=/; max-age=0";

                const rootCookies = document.cookie;
                location.assign("/admin/panel?x=1#hash");

                return {
                    rootCookies,
                    adminCookies: document.cookie,
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["rootCookies"], "gamma=3");
    assert_eq!(value["adminCookies"], "beta=2; gamma=3");
    assert_eq!(value["href"], "https://rom.local/admin/panel?x=1#hash");
    assert_eq!(value["pathname"], "/admin/panel");
    assert_eq!(value["search"], "?x=1");
    assert_eq!(value["hash"], "#hash");
}

#[test]
fn supports_cookie_roundtrip_between_document_and_fetch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for request_index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
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

            let request = String::from_utf8_lossy(&buffer);

            if request_index == 0 {
                assert!(request.contains("GET /app/set HTTP/1.1"));
                assert!(request.contains("cookie: init=1"));

                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Set-Cookie: session=abc; Path=/app\r\n",
                    "Set-Cookie: token=secret; Path=/app; HttpOnly\r\n",
                    "Set-Cookie: prefs=light; Path=/prefs\r\n",
                    "Content-Length: 2\r\n",
                    "\r\n",
                    "ok"
                );

                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            assert!(request.contains("GET /app/echo HTTP/1.1"));
            assert!(request.contains("cookie: init=1; session=abc; token=secret"));
            assert!(!request.contains("prefs=light"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 4\r\n",
                "\r\n",
                "done"
            );

            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        href: format!("http://{address}/app/index.html"),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = r#"
        (async () => {
            document.cookie = "init=1; path=/app";
            await fetch("/app/set");
            const response = await fetch("/app/echo");

            return {
                documentCookie: document.cookie,
                text: await response.text(),
            };
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["documentCookie"], "init=1; session=abc");
    assert_eq!(value["text"], "done");
}

#[test]
fn strips_forbidden_cookie_headers_and_hides_set_cookie_from_responses() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for request_index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
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

            let request = String::from_utf8_lossy(&buffer);

            if request_index == 0 {
                assert!(request.contains("GET /seed HTTP/1.1"));
                assert!(request.contains("cookie: seed=1"));
                assert!(!request.contains("manual=blocked"));

                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Set-Cookie: fresh=2; Path=/\r\n",
                    "Content-Type: text/plain\r\n",
                    "Content-Length: 4\r\n",
                    "\r\n",
                    "seed"
                );

                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            assert!(request.contains("GET /echo HTTP/1.1"));
            assert!(request.contains("cookie: seed=1; fresh=2"));
            assert!(!request.contains("manual=blocked"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: text/plain\r\n",
                "Content-Length: 4\r\n",
                "\r\n",
                "done"
            );

            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        href: format!("http://{address}/index.html"),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = r#"
        (async () => {
            document.cookie = "seed=1; path=/";
            const seeded = await fetch("/seed", {
                headers: {
                    cookie: "manual=blocked",
                },
            });
            const echoed = await fetch("/echo", {
                headers: {
                    cookie: "manual=blocked",
                },
            });

            return {
                seededVisibleSetCookie: seeded.headers.get("set-cookie"),
                seededText: await seeded.text(),
                echoedText: await echoed.text(),
                documentCookie: document.cookie,
            };
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();

    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["seededVisibleSetCookie"], serde_json::Value::Null);
    assert_eq!(value["seededText"], "seed");
    assert_eq!(value["echoedText"], "done");
    assert_eq!(value["documentCookie"], "seed=1; fresh=2");
}
