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
fn enforces_request_body_method_guards_and_null_body_surfaces() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                let getBodyError = "";
                try {
                    new Request("https://rom.local/get", {
                        method: "GET",
                        body: "blocked",
                    });
                } catch (error) {
                    getBodyError = String(error.message ?? error);
                }

                let headBodyError = "";
                try {
                    await fetch("https://rom.local/head", {
                        method: "HEAD",
                        body: "blocked",
                    });
                } catch (error) {
                    headBodyError = String(error.message ?? error);
                }

                const emptyRequest = new Request("https://rom.local/no-body");
                const explicitEmptyRequest = new Request("https://rom.local/empty", {
                    method: "POST",
                    body: "",
                });
                const emptyResponse = new Response();
                const explicitEmptyResponse = new Response("");
                const clonedEmptyRequest = emptyRequest.clone();
                const clonedEmptyResponse = emptyResponse.clone();

                return {
                    getBodyError,
                    headBodyError,
                    emptyRequestBodyIsNull: emptyRequest.body === null,
                    explicitEmptyRequestBodyIsStream: explicitEmptyRequest.body instanceof ReadableStream,
                    emptyResponseBodyIsStream: emptyResponse.body instanceof ReadableStream,
                    explicitEmptyResponseBodyIsStream: explicitEmptyResponse.body instanceof ReadableStream,
                    clonedEmptyRequestBodyIsNull: clonedEmptyRequest.body === null,
                    clonedEmptyResponseBodyIsStream: clonedEmptyResponse.body instanceof ReadableStream,
                    explicitEmptyRequestText: await explicitEmptyRequest.text(),
                    explicitEmptyResponseText: await explicitEmptyResponse.text(),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["getBodyError"],
        "Failed to construct 'Request': Request with GET/HEAD method cannot have body."
    );
    assert_eq!(
        value["headBodyError"],
        "Failed to construct 'Request': Request with GET/HEAD method cannot have body."
    );
    assert_eq!(value["emptyRequestBodyIsNull"], true);
    assert_eq!(value["explicitEmptyRequestBodyIsStream"], true);
    assert_eq!(value["emptyResponseBodyIsStream"], true);
    assert_eq!(value["explicitEmptyResponseBodyIsStream"], true);
    assert_eq!(value["clonedEmptyRequestBodyIsNull"], true);
    assert_eq!(value["clonedEmptyResponseBodyIsStream"], true);
    assert_eq!(value["explicitEmptyRequestText"], "");
    assert_eq!(value["explicitEmptyResponseText"], "");
}

#[test]
fn rejects_reusing_consumed_request_bodies_without_override() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const source = new Request("https://rom.local/source", {
                    method: "POST",
                    body: "payload",
                });
                await source.text();

                let constructorError = "";
                try {
                    new Request(source);
                } catch (error) {
                    constructorError = String(error.message ?? error);
                }

                let fetchError = "";
                try {
                    await fetch(source);
                } catch (error) {
                    fetchError = String(error.message ?? error);
                }

                const overrideSource = new Request("https://rom.local/override", {
                    method: "POST",
                    body: "payload",
                });
                await overrideSource.text();
                const rebuilt = new Request(overrideSource, {
                    method: "POST",
                    body: "fresh",
                });

                const lockedSource = new Request("https://rom.local/locked", {
                    method: "POST",
                    body: "payload",
                });
                const reader = lockedSource.body.getReader();

                let lockedError = "";
                try {
                    new Request(lockedSource);
                } catch (error) {
                    lockedError = String(error.message ?? error);
                }
                reader.releaseLock();

                return {
                    constructorError,
                    fetchError,
                    rebuiltText: await rebuilt.text(),
                    lockedError,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["constructorError"],
        "Failed to construct 'Request': Cannot construct a Request from a Request with a used body."
    );
    assert_eq!(
        value["fetchError"],
        "Failed to construct 'Request': Cannot construct a Request from a Request with a used body."
    );
    assert_eq!(value["rebuiltText"], "fresh");
    assert_eq!(
        value["lockedError"],
        "Failed to construct 'Request': Cannot construct a Request from a Request with a used body."
    );
}

#[test]
fn validates_request_method_and_enum_values() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const standardMethod = new Request("https://rom.local/standard", {
                    method: "post",
                }).method;
                const customMethod = new Request("https://rom.local/custom", {
                    method: "patch",
                }).method;

                let invalidMethodError = "";
                try {
                    new Request("https://rom.local/invalid-method", {
                        method: "bad method",
                    });
                } catch (error) {
                    invalidMethodError = String(error.message ?? error);
                }

                let forbiddenMethodError = "";
                try {
                    new Request("https://rom.local/trace", {
                        method: "TRACE",
                    });
                } catch (error) {
                    forbiddenMethodError = String(error.message ?? error);
                }

                let invalidCredentialsError = "";
                try {
                    new Request("https://rom.local/credentials", {
                        credentials: "always",
                    });
                } catch (error) {
                    invalidCredentialsError = String(error.message ?? error);
                }

                let invalidModeError = "";
                try {
                    new Request("https://rom.local/mode", {
                        mode: "navigate",
                    });
                } catch (error) {
                    invalidModeError = String(error.message ?? error);
                }

                let invalidRedirectError = "";
                try {
                    new Request("https://rom.local/redirect", {
                        redirect: "follow-manual",
                    });
                } catch (error) {
                    invalidRedirectError = String(error.message ?? error);
                }

                return {
                    standardMethod,
                    customMethod,
                    invalidMethodError,
                    forbiddenMethodError,
                    invalidCredentialsError,
                    invalidModeError,
                    invalidRedirectError,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["standardMethod"], "POST");
    assert_eq!(value["customMethod"], "patch");
    assert_eq!(
        value["invalidMethodError"],
        "Failed to construct 'Request': 'bad method' is not a valid HTTP method."
    );
    assert_eq!(
        value["forbiddenMethodError"],
        "Failed to construct 'Request': 'TRACE' HTTP method is unsupported."
    );
    assert_eq!(
        value["invalidCredentialsError"],
        "Failed to construct 'Request': 'always' is not a valid enum value of type RequestCredentials."
    );
    assert_eq!(
        value["invalidModeError"],
        "Failed to construct 'Request': 'navigate' is not a valid enum value of type RequestMode."
    );
    assert_eq!(
        value["invalidRedirectError"],
        "Failed to construct 'Request': 'follow-manual' is not a valid enum value of type RequestRedirect."
    );
}

#[test]
fn applies_bodyinit_content_types_and_string_coercion() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const textRequest = new Request("https://rom.local/text", {
                    method: "POST",
                    body: "hello",
                });
                const paramsRequest = new Request("https://rom.local/params", {
                    method: "POST",
                    body: new URLSearchParams([
                        ["alpha", "1"],
                        ["beta", "two words"],
                    ]),
                });
                const objectRequest = new Request("https://rom.local/object", {
                    method: "POST",
                    body: { answer: 42 },
                });
                const textResponse = new Response("hello");

                return {
                    textRequestContentType: textRequest.headers.get("content-type"),
                    textRequestBody: await textRequest.text(),
                    paramsRequestContentType: paramsRequest.headers.get("content-type"),
                    paramsRequestBody: await paramsRequest.text(),
                    objectRequestBody: await objectRequest.text(),
                    textResponseContentType: textResponse.headers.get("content-type"),
                    textResponseBody: await textResponse.text(),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["textRequestContentType"], "text/plain;charset=UTF-8");
    assert_eq!(value["textRequestBody"], "hello");
    assert_eq!(
        value["paramsRequestContentType"],
        "application/x-www-form-urlencoded;charset=UTF-8"
    );
    assert_eq!(value["paramsRequestBody"], "alpha=1&beta=two+words");
    assert_eq!(value["objectRequestBody"], "[object Object]");
    assert_eq!(value["textResponseContentType"], "text/plain;charset=UTF-8");
    assert_eq!(value["textResponseBody"], "hello");
}

#[test]
fn rejects_non_safelisted_methods_in_no_cors_mode() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const allowedMethod = new Request("https://rom.local/post", {
                    method: "POST",
                    mode: "no-cors",
                }).method;

                let putError = "";
                try {
                    new Request("https://rom.local/put", {
                        method: "PUT",
                        mode: "no-cors",
                    });
                } catch (error) {
                    putError = String(error.message ?? error);
                }

                let patchError = "";
                try {
                    await fetch("https://rom.local/patch", {
                        method: "PATCH",
                        mode: "no-cors",
                    });
                } catch (error) {
                    patchError = String(error.message ?? error);
                }

                return {
                    allowedMethod,
                    putError,
                    patchError,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["allowedMethod"], "POST");
    assert_eq!(
        value["putError"],
        "Failed to construct 'Request': 'PUT' is unsupported in no-cors mode."
    );
    assert_eq!(
        value["patchError"],
        "Failed to construct 'Request': 'PATCH' is unsupported in no-cors mode."
    );
}

#[test]
fn strips_non_safelisted_headers_in_no_cors_mode() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http_request(&mut stream);

        assert!(request.contains("POST /opaque-headers HTTP/1.1"));
        assert!(request.contains("accept-language: en-US"));
        assert!(!request.contains("x-rom-test: blocked"));
        assert!(!request.contains("content-type: application/json"));
        assert!(!request.contains("content-type:"));

        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: text/plain\r\n",
            "Content-Length: 2\r\n",
            "\r\n",
            "ok"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const response = await fetch("http://{address}/opaque-headers", {{
                method: "POST",
                mode: "no-cors",
                headers: {{
                    "accept-language": "en-US",
                    "content-type": "application/json",
                    "x-rom-test": "blocked",
                }},
                body: "payload",
            }});

            return {{
                type: response.type,
                status: response.status,
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["type"], "opaque");
    assert_eq!(value["status"], 0);
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

#[test]
fn supports_no_cors_opaque_response_semantics() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_http_request(&mut stream);

            if request.contains("GET /opaque HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Content-Type: text/plain\r\n",
                    "X-Hidden: secret\r\n",
                    "Content-Length: 6\r\n",
                    "\r\n",
                    "opaque"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                continue;
            }

            if request.contains("GET /manual HTTP/1.1") {
                let response = concat!(
                    "HTTP/1.1 302 Found\r\n",
                    "Location: /redirected\r\n",
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
                    "Location: /blocked\r\n",
                    "Content-Length: 0\r\n",
                    "\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        }
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            const opaque = await fetch("http://{address}/opaque", {{ mode: "no-cors" }});
            const clone = opaque.clone();
            const manual = await fetch("http://{address}/manual", {{
                mode: "no-cors",
                redirect: "manual",
            }});

            let errorMessage = "";
            try {{
                await fetch("http://{address}/error", {{
                    mode: "no-cors",
                    redirect: "error",
                }});
            }} catch (error) {{
                errorMessage = String(error.message ?? error);
            }}

            let invalidStatusName = "";
            try {{
                new Response("x", {{ status: 0 }});
            }} catch (error) {{
                invalidStatusName = error.name;
            }}

            return {{
                opaqueType: opaque.type,
                opaqueStatus: opaque.status,
                opaqueOk: opaque.ok,
                opaqueUrl: opaque.url,
                opaqueBodyIsNull: opaque.body === null,
                opaqueHeaderCount: Array.from(opaque.headers).length,
                opaqueContentType: opaque.headers.get("content-type"),
                opaqueText: await opaque.text(),
                opaqueBodyUsed: opaque.bodyUsed,
                cloneBodyIsNull: clone.body === null,
                cloneText: await clone.text(),
                manualType: manual.type,
                manualStatus: manual.status,
                manualBodyIsNull: manual.body === null,
                errorMessage,
                invalidStatusName,
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["opaqueType"], "opaque");
    assert_eq!(value["opaqueStatus"], 0);
    assert_eq!(value["opaqueOk"], false);
    assert_eq!(value["opaqueUrl"], "");
    assert_eq!(value["opaqueBodyIsNull"], true);
    assert_eq!(value["opaqueHeaderCount"], 0);
    assert_eq!(value["opaqueContentType"], serde_json::Value::Null);
    assert_eq!(value["opaqueText"], "");
    assert_eq!(value["opaqueBodyUsed"], true);
    assert_eq!(value["cloneBodyIsNull"], true);
    assert_eq!(value["cloneText"], "");
    assert_eq!(value["manualType"], "opaqueredirect");
    assert_eq!(value["manualStatus"], 0);
    assert_eq!(value["manualBodyIsNull"], true);
    assert_eq!(value["errorMessage"], "Failed to fetch");
    assert_eq!(value["invalidStatusName"], "RangeError");
}

#[test]
fn supports_readable_stream_cancel_and_tee_semantics() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const response = new Response("tee-body");
                const [left, right] = response.body.tee();

                let cloneError = "";
                try {
                    response.clone();
                } catch (error) {
                    cloneError = String(error.message ?? error);
                }

                const leftChunk = await left.getReader().read();
                const rightChunk = await right.getReader().read();

                const cancelResponse = new Response("cancel-me");
                await cancelResponse.body.cancel();

                let cancelTextError = "";
                try {
                    await cancelResponse.text();
                } catch (error) {
                    cancelTextError = String(error.message ?? error);
                }

                const lockedResponse = new Response("locked");
                const lockedReader = lockedResponse.body.getReader();
                let lockedCancelName = "";
                try {
                    await lockedResponse.body.cancel();
                } catch (error) {
                    lockedCancelName = error.name;
                }
                lockedReader.releaseLock();

                const cloneable = new Request("https://rom.local/upload", {
                    method: "POST",
                    body: "cloneable",
                });
                const cloneableReader = cloneable.body.getReader();
                cloneableReader.releaseLock();
                const cloned = cloneable.clone();

                const disturbed = new Response("disturbed");
                const disturbedReader = disturbed.body.getReader();
                await disturbedReader.read();
                disturbedReader.releaseLock();
                let disturbedTeeName = "";
                try {
                    disturbed.body.tee();
                } catch (error) {
                    disturbedTeeName = error.name;
                }

                return {
                    leftText: new TextDecoder().decode(leftChunk.value),
                    rightText: new TextDecoder().decode(rightChunk.value),
                    cloneError,
                    cancelBodyUsed: cancelResponse.bodyUsed,
                    cancelTextError,
                    lockedCancelName,
                    clonedText: await cloned.text(),
                    originalText: await cloneable.text(),
                    disturbedTeeName,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["leftText"], "tee-body");
    assert_eq!(value["rightText"], "tee-body");
    assert_eq!(
        value["cloneError"],
        "Failed to execute 'clone' on 'Response': body has already been used."
    );
    assert_eq!(value["cancelBodyUsed"], true);
    assert_eq!(value["cancelTextError"], "Body has already been read.");
    assert_eq!(value["lockedCancelName"], "TypeError");
    assert_eq!(value["clonedText"], "cloneable");
    assert_eq!(value["originalText"], "cloneable");
    assert_eq!(value["disturbedTeeName"], "TypeError");
}
