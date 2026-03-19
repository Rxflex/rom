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
fn supports_readable_stream_body_consumption() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const response = new Response("stream-body", {
                    headers: { "content-type": "text/plain" },
                });
                const reader = response.body.getReader();
                const first = await reader.read();
                const second = await reader.read();
                reader.releaseLock();

                let bodyTextError = "";
                try {
                    await response.text();
                } catch (error) {
                    bodyTextError = String(error.message ?? error);
                }

                const request = new Request("https://rom.local/upload", {
                    method: "POST",
                    body: "request-body",
                });
                const requestReader = request.body.getReader();
                const requestChunk = await requestReader.read();

                return {
                    isReadableStream: response.body instanceof ReadableStream,
                    firstChunk: new TextDecoder().decode(first.value),
                    firstDone: first.done,
                    secondDone: second.done,
                    responseBodyUsed: response.bodyUsed,
                    bodyTextError,
                    requestBodyUsed: request.bodyUsed,
                    requestChunk: new TextDecoder().decode(requestChunk.value),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["isReadableStream"], true);
    assert_eq!(value["firstChunk"], "stream-body");
    assert_eq!(value["firstDone"], false);
    assert_eq!(value["secondDone"], true);
    assert_eq!(value["responseBodyUsed"], true);
    assert_eq!(value["bodyTextError"], "Body has already been read.");
    assert_eq!(value["requestBodyUsed"], true);
    assert_eq!(value["requestChunk"], "request-body");
}

#[test]
fn supports_redirect_modes() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);

            if request.contains("GET /follow HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 302 Found\r\n",
                    "Location: /final\r\n",
                    "Content-Length: 0\r\n",
                    "\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            if request.contains("GET /final HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Content-Type: text/plain\r\n",
                    "Content-Length: 5\r\n",
                    "\r\n",
                    "final"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            if request.contains("GET /manual HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 302 Found\r\n",
                    "Location: /manual-target\r\n",
                    "Content-Length: 0\r\n",
                    "\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            if request.contains("GET /error HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 301 Moved Permanently\r\n",
                    "Location: /error-target\r\n",
                    "Content-Length: 0\r\n",
                    "\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig {
        href: format!("http://{address}/"),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let script = r#"
        (async () => {
            const followed = await fetch("/follow");
            const manual = await fetch("/manual", { redirect: "manual" });

            let errorMessage = "";
            try {
                await fetch("/error", { redirect: "error" });
            } catch (error) {
                errorMessage = String(error.message ?? error);
            }

            return {
                followedType: followed.type,
                followedRedirected: followed.redirected,
                followedUrl: followed.url,
                followedBody: await followed.text(),
                manualType: manual.type,
                manualStatus: manual.status,
                manualUrl: manual.url,
                errorMessage,
            };
        })()
    "#;

    let result = runtime.eval_async_as_string(script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["followedType"], "basic");
    assert_eq!(value["followedRedirected"], true);
    assert_eq!(value["followedUrl"], format!("http://{address}/final"));
    assert_eq!(value["followedBody"], "final");
    assert_eq!(value["manualType"], "opaqueredirect");
    assert_eq!(value["manualStatus"], 0);
    assert_eq!(value["manualUrl"], "");
    assert_eq!(value["errorMessage"], "Failed to fetch");
}
