mod bridge;
mod compat;
mod config;
mod error;
mod fingerprintjs;
mod runtime;
#[cfg(test)]
mod tests_adjacent_html;
#[cfg(test)]
mod tests_character_data;
#[cfg(test)]
mod tests_classlist;
#[cfg(test)]
mod tests_collections;
#[cfg(test)]
mod tests_comments;
#[cfg(test)]
mod tests_cookies;
#[cfg(test)]
mod tests_cors;
#[cfg(test)]
mod tests_css;
#[cfg(test)]
mod tests_dataset;
#[cfg(test)]
mod tests_document_surface;
#[cfg(test)]
mod tests_dom_mutations;
#[cfg(test)]
mod tests_dom_navigation;
#[cfg(test)]
mod tests_events;
#[cfg(test)]
mod tests_eventsource;
#[cfg(test)]
mod tests_fetch_semantics;
#[cfg(test)]
mod tests_file_reader;
#[cfg(test)]
mod tests_fragments;
#[cfg(test)]
mod tests_history;
#[cfg(test)]
mod tests_innerhtml;
#[cfg(test)]
mod tests_layout_observers;
#[cfg(test)]
mod tests_messaging;
#[cfg(test)]
mod tests_mutation_observer;
#[cfg(test)]
mod tests_navigator;
#[cfg(test)]
mod tests_node_equality;
#[cfg(test)]
mod tests_node_helpers;
#[cfg(test)]
mod tests_normalize;
#[cfg(test)]
mod tests_parsing;
#[cfg(test)]
mod tests_performance;
#[cfg(test)]
mod tests_raw_risk_surface;
#[cfg(test)]
mod tests_selectors;
#[cfg(test)]
mod tests_split_text;
#[cfg(test)]
mod tests_timers;
#[cfg(test)]
mod tests_viewport;
#[cfg(test)]
mod tests_webcrypto;
#[cfg(test)]
mod tests_websocket;

pub use bridge::{
    BridgeCommand, BridgeRequest, BridgeResponse, execute_bridge_request,
    execute_bridge_request_json, parse_bridge_request,
};
pub use compat::{
    CanvasSurface, FingerprintCanvas, FingerprintMedia, FingerprintObservers, FingerprintProbe,
    FingerprintScreen, FingerprintStorage, GlobalSurface, NavigatorSurface, ObserverSurface,
    SurfaceSnapshot,
};
pub use config::RuntimeConfig;
pub use error::{Result, RuntimeError};
pub use fingerprintjs::{
    ComponentError, FingerprintJsHarnessDiff, FingerprintJsHarnessReport, HarnessError,
};
pub use runtime::RomRuntime;

#[cfg(test)]
mod tests {
    use crate::{RomRuntime, RuntimeConfig};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn boots_browser_globals() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result: bool = runtime
            .eval(
                r#"
                window === self &&
                document.defaultView === window &&
                navigator.webdriver === false &&
                location.href === "https://rom.local/"
                "#,
            )
            .unwrap();

        assert!(result);
    }

    #[test]
    fn supports_basic_dom_workflow() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result: bool = runtime
            .eval(
                r##"
                const root = document.createElement("div");
                root.id = "root";
                document.body.appendChild(root);
                document.querySelector("#root") === root &&
                document.getElementById("root") === root &&
                document.querySelectorAll("#root").length === 1
                "##,
            )
            .unwrap();

        assert!(result);
    }

    #[test]
    fn preserves_globals_across_runtime_eval_calls() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let first = runtime
            .eval_as_string("globalThis.__rom_eval_value = 42; 'ok'")
            .unwrap();
        let second = runtime
            .eval_as_string("String(globalThis.__rom_eval_value)")
            .unwrap();

        assert_eq!(first, "ok");
        assert_eq!(second, "42");
    }

    #[test]
    fn exposes_compatibility_stubs() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result: bool = runtime
            .eval(
                r#"
                typeof MutationObserver === "function" &&
                typeof ResizeObserver === "function" &&
                typeof crypto.getRandomValues === "function" &&
                typeof TextEncoder === "function" &&
                typeof TextDecoder === "function" &&
                typeof document.createElement("canvas").getContext("2d").measureText("gom").width === "number" &&
                typeof AudioContext === "function"
                "#,
            )
            .unwrap();

        assert!(result);
    }

    #[test]
    fn produces_surface_snapshot() {
        let config = RuntimeConfig::default();
        let runtime = RomRuntime::new(config.clone()).unwrap();
        let snapshot = runtime.surface_snapshot().unwrap();

        assert!(snapshot.globals.window);
        assert!(snapshot.globals.document);
        assert!(snapshot.globals.text_encoder);
        assert!(snapshot.canvas.has_canvas);
        assert!(snapshot.canvas.has_2d_context);
        assert_eq!(snapshot.navigator.user_agent, config.user_agent);
    }

    #[test]
    fn produces_fingerprint_probe() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let probe = runtime.fingerprint_probe().unwrap();

        assert!(probe.storage.local_storage);
        assert!(probe.storage.session_storage);
        assert!(probe.media.match_media);
        assert!(probe.media.audio_context);
        assert!(probe.canvas.has_2d_context);
        assert!(probe.observers.mutation);
        assert_eq!(probe.max_touch_points, 0);
    }

    #[test]
    fn runs_fingerprintjs_harness() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let report = runtime.run_fingerprintjs_harness().unwrap();

        assert_eq!(runtime.fingerprintjs_version(), "5.1.0");
        assert!(report.ok);
        assert_eq!(report.version.as_deref(), Some("5.1.0"));
        assert!(report.visitor_id.is_some());
        assert!(report.confidence_score.is_some());
        assert!(report.component_count > 0);
    }

    #[test]
    fn matches_default_fingerprintjs_snapshot() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let report = runtime.run_fingerprintjs_harness().unwrap();
        let baseline = RomRuntime::default_fingerprintjs_harness_snapshot();
        let diff = report.diff(&baseline).without_identity();

        assert!(diff.is_empty(), "unexpected fingerprintjs diff: {diff:?}");
    }

    #[test]
    fn matches_chromium_reference_on_stable_fields() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let report = runtime.run_fingerprintjs_harness().unwrap();
        let baseline = RomRuntime::chromium_fingerprintjs_harness_snapshot();
        let diff = report.diff(&baseline).without_identity();

        assert!(
            diff.is_empty(),
            "unexpected chromium reference diff: {diff:?}"
        );
    }

    #[test]
    #[ignore = "debug utility for inspecting the current harness report"]
    fn debug_prints_fingerprintjs_harness_report() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let report = runtime.run_fingerprintjs_harness().unwrap();

        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }

    #[test]
    fn supports_fetch_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
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

            assert!(request.contains("POST /echo HTTP/1.1"));
            assert!(request.contains("x-rom-test: yes"));
            assert!(request.contains("{\"hello\":\"world\"}"));

            let response = concat!(
                "HTTP/1.1 200 OK\r\n",
                "Content-Type: application/json\r\n",
                "X-Rom-Reply: ok\r\n",
                "Content-Length: 17\r\n",
                "\r\n",
                "{\"received\":true}"
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
                const request = new Request("/echo", {
                    method: "POST",
                    headers: new Headers({
                        "content-type": "application/json",
                        "x-rom-test": "yes",
                    }),
                    body: JSON.stringify({ hello: "world" }),
                });
                const response = await fetch(request);
                const data = await response.json();

                return {
                    ok: response.ok,
                    status: response.status,
                    contentType: response.headers.get("content-type"),
                    replyHeader: response.headers.get("x-rom-reply"),
                    received: data.received === true,
                };
            })()
            "#
        .to_string();

        let result = runtime.eval_async_as_string(&script).unwrap();

        server.join().unwrap();
        assert_eq!(
            result,
            r#"{"ok":true,"status":200,"contentType":"application/json","replyHeader":"ok","received":true}"#
        );
    }

    #[test]
    fn supports_fetch_abort_signal() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result = runtime
            .eval_async_as_string(
                r#"
                (async () => {
                    const controller = new AbortController();
                    controller.abort(new Error("aborted by test"));

                    try {
                        await fetch("http://127.0.0.1:1/never", { signal: controller.signal });
                        return "unexpected";
                    } catch (error) {
                        return String(error.message ?? error);
                    }
                })()
                "#,
            )
            .unwrap();

        assert_eq!(result, "aborted by test");
    }

    #[test]
    fn supports_url_and_search_params() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result: bool = runtime
            .eval(
                r#"
                const url = new URL("/api/items?x=1", "https://example.com/root");
                url.searchParams.append("y", "2");
                url.hostname = "api.example.com";
                url.port = "8443";
                url.hash = "done";

                url.href === "https://api.example.com:8443/api/items?x=1&y=2#done" &&
                url.origin === "https://api.example.com:8443" &&
                url.searchParams.get("x") === "1" &&
                url.searchParams.get("y") === "2" &&
                new Request("/relative/path").url === "https://rom.local/relative/path"
                "#,
            )
            .unwrap();

        assert!(result);
    }

    #[test]
    fn supports_body_helpers() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result = runtime
            .eval_async_as_string(
                r#"
                (async () => {
                    const upload = new FormData();
                    upload.append("alpha", "1");
                    upload.append("file", new File(["payload"], "payload.txt", { type: "text/plain" }));

                    const request = new Request("https://rom.local/upload", {
                        method: "POST",
                        body: upload,
                    });
                    const requestForm = await request.clone().formData();
                    const requestBlob = await request.blob();
                    let secondReadMessage = "";

                    try {
                        await request.text();
                    } catch (error) {
                        secondReadMessage = String(error.message ?? error);
                    }

                    const responseForm = await new Response("x=1&y=two", {
                        headers: {
                            "content-type": "application/x-www-form-urlencoded;charset=UTF-8",
                        },
                    }).formData();

                    const responseBlob = await new Response("hello", {
                        headers: { "content-type": "text/plain;charset=UTF-8" },
                    }).blob();

                    return {
                        requestContentType: request.headers.get("content-type"),
                        requestAlpha: requestForm.get("alpha"),
                        requestFileName: requestForm.get("file").name,
                        requestFileText: await requestForm.get("file").text(),
                        requestBlobType: requestBlob.type,
                        requestBodyUsed: request.bodyUsed,
                        secondReadMessage,
                        responseX: responseForm.get("x"),
                        responseY: responseForm.get("y"),
                        responseBlobType: responseBlob.type,
                        responseBlobText: await responseBlob.text(),
                    };
                })()
                "#,
            )
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert!(
            value["requestContentType"]
                .as_str()
                .unwrap()
                .starts_with("multipart/form-data; boundary=")
        );
        assert_eq!(value["requestAlpha"], "1");
        assert_eq!(value["requestFileName"], "payload.txt");
        assert_eq!(value["requestFileText"], "payload");
        assert_eq!(value["requestBlobType"], "multipart/form-data");
        assert_eq!(value["requestBodyUsed"], true);
        assert_eq!(value["secondReadMessage"], "Body has already been read.");
        assert_eq!(value["responseX"], "1");
        assert_eq!(value["responseY"], "two");
        assert_eq!(value["responseBlobType"], "text/plain");
        assert_eq!(value["responseBlobText"], "hello");
    }

    #[test]
    fn supports_blob_object_urls() {
        let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
        let result = runtime
            .eval_async_as_string(
                r#"
                (async () => {
                    const blob = new Blob(["rom-object-url"], { type: "text/plain" });
                    const first = URL.createObjectURL(blob);
                    const second = URL.createObjectURL(blob);
                    const response = await fetch(first);
                    const text = await response.text();
                    const type = response.headers.get("content-type");
                    URL.revokeObjectURL(first);

                    let revokedMessage = "";
                    try {
                        await fetch(first);
                    } catch (error) {
                        revokedMessage = String(error.message ?? error);
                    }

                    return {
                        first,
                        second,
                        unique: first !== second,
                        text,
                        type,
                        revokedMessage,
                    };
                })()
                "#,
            )
            .unwrap();

        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        let first = value["first"].as_str().unwrap();
        let second = value["second"].as_str().unwrap();

        assert!(first.starts_with("blob:https://rom.local/"));
        assert!(second.starts_with("blob:https://rom.local/"));
        assert_ne!(first, second);
        assert_eq!(value["unique"], true);
        assert_eq!(value["text"], "rom-object-url");
        assert_eq!(value["type"], "text/plain");
        assert_eq!(value["revokedMessage"], "Failed to fetch");
    }

    #[test]
    fn supports_fetch_form_data_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let server = thread::spawn(move || {
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

            assert!(request.contains("POST /upload HTTP/1.1"));
            assert!(request.contains("content-type: multipart/form-data; boundary="));
            assert!(request.contains("name=\"alpha\""));
            assert!(request.contains("name=\"file\"; filename=\"payload.txt\""));
            assert!(request.contains("Content-Type: text/plain"));
            assert!(request.contains("payload"));

            let response = concat!(
                "HTTP/1.1 204 No Content\r\n",
                "Content-Length: 0\r\n",
                "\r\n"
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
                const body = new FormData();
                body.append("alpha", "1");
                body.append("file", new File(["payload"], "payload.txt", { type: "text/plain" }));
                const response = await fetch("/upload", {
                    method: "POST",
                    body,
                });

                return {
                    ok: response.ok,
                    status: response.status,
                };
            })()
            "#
        .to_string();

        let result = runtime.eval_async_as_string(&script).unwrap();

        server.join().unwrap();
        assert_eq!(result, r#"{"ok":true,"status":204}"#);
    }
}
