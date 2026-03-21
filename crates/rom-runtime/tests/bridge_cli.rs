use serde_json::{Value, json};
use std::{
    io::Read,
    io::Write,
    net::TcpListener,
    process::{Command, Stdio},
    thread,
};

fn run_bridge(payload: Value) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rom_bridge"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    serde_json::to_writer(child.stdin.as_mut().unwrap(), &payload).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();

    let output = child.wait_with_output().unwrap();
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.success(), value)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }

        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8_lossy(&buffer).into_owned()
}

#[test]
fn bridge_evaluates_async_scripts() {
    let (success, value) = run_bridge(json!({
        "command": "eval-async",
        "script": r#"
            (async () => {
                await Promise.resolve();
                return location.href;
            })()
        "#,
    }));

    assert!(success);
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"], "https://rom.local/");
}

#[test]
fn bridge_returns_surface_snapshot() {
    let (success, value) = run_bridge(json!({
        "command": "surface-snapshot",
        "config": {
            "href": "https://example.test/app",
            "user_agent": "ROM Test Agent",
        }
    }));

    assert!(success);
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"]["globals"]["window"], true);
    assert_eq!(value["result"]["navigator"]["user_agent"], "ROM Test Agent");
}

#[test]
fn bridge_reports_protocol_errors() {
    let (success, value) = run_bridge(json!({
        "command": "eval",
    }));

    assert!(!success);
    assert_eq!(value["ok"], false);
    assert_eq!(value["error"], "Missing script for bridge command.");
}

#[test]
fn bridge_preserves_cookie_store_between_invocations() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for request_index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);

            if request_index == 0 {
                assert!(request.contains("GET /seed HTTP/1.1"));
                assert!(request.contains("cookie: init=1"));
                stream
                    .write_all(
                        concat!(
                            "HTTP/1.1 200 OK\r\n",
                            "Set-Cookie: session=bridge; Path=/\r\n",
                            "Content-Length: 4\r\n",
                            "\r\n",
                            "seed"
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                stream.flush().unwrap();
                continue;
            }

            assert!(request.contains("GET /echo HTTP/1.1"));
            assert!(request.contains("cookie: init=1; session=bridge"));
            stream
                .write_all(
                    concat!(
                        "HTTP/1.1 200 OK\r\n",
                        "Content-Length: 4\r\n",
                        "\r\n",
                        "echo"
                    )
                    .as_bytes(),
                )
                .unwrap();
            stream.flush().unwrap();
        }
    });

    let (success_first, first) = run_bridge(json!({
        "command": "eval-async",
        "config": {
            "href": format!("http://{address}/index.html"),
        },
        "script": r#"
            (async () => {
                document.cookie = "init=1; path=/";
                const response = await fetch("/seed");
                return {
                    text: await response.text(),
                    documentCookie: document.cookie,
                };
            })()
        "#,
    }));

    assert!(success_first);
    assert_eq!(first["ok"], true);
    assert_eq!(
        first["result"],
        "{\"text\":\"seed\",\"documentCookie\":\"init=1; session=bridge\"}"
    );

    let cookie_store = first["state"]["cookie_store"].as_str().unwrap().to_owned();
    assert!(!cookie_store.is_empty());

    let (success_second, second) = run_bridge(json!({
        "command": "eval-async",
        "config": {
            "href": format!("http://{address}/index.html"),
            "cookie_store": cookie_store,
        },
        "script": r#"
            (async () => {
                const response = await fetch("/echo");
                return {
                    text: await response.text(),
                    documentCookie: document.cookie,
                };
            })()
        "#,
    }));

    server.join().unwrap();

    assert!(success_second);
    assert_eq!(second["ok"], true);
    assert_eq!(
        second["result"],
        "{\"text\":\"echo\",\"documentCookie\":\"init=1; session=bridge\"}"
    );
}
