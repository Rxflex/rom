use crate::{RomRuntime, RuntimeConfig};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn supports_structured_clone_and_message_channel() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const cyclic = { name: "root" };
                cyclic.self = cyclic;
                const cloned = structuredClone(cyclic);

                let cloneError = "";
                try {
                    structuredClone(() => {});
                } catch (error) {
                    cloneError = String(error.message ?? error);
                }

                const channel = new MessageChannel();
                const original = {
                    nested: { value: 1 },
                    buffer: new Uint8Array([1, 2, 3]).buffer,
                };

                const message = await new Promise((resolve) => {
                    channel.port2.onmessage = (event) => {
                        const bytes = Array.from(new Uint8Array(event.data.buffer));
                        event.data.nested.value = 9;
                        resolve({
                            isMessageEvent: event instanceof MessageEvent,
                            sourceMatches: event.source === channel.port1,
                            bytes,
                            nestedValue: event.data.nested.value,
                        });
                    };

                    channel.port1.postMessage(original);
                });

                return {
                    clonedSelf: cloned !== cyclic && cloned.self === cloned,
                    originalUnchanged: original.nested.value === 1,
                    cloneError,
                    message,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["clonedSelf"], true);
    assert_eq!(value["originalUnchanged"], true);
    assert_eq!(value["cloneError"], "DataCloneError");
    assert_eq!(value["message"]["isMessageEvent"], true);
    assert_eq!(value["message"]["sourceMatches"], true);
    assert_eq!(value["message"]["bytes"], serde_json::json!([1, 2, 3]));
    assert_eq!(value["message"]["nestedValue"], 9);
}

#[test]
fn supports_structured_clone_for_error_objects() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const original = new TypeError("boom");
                original.code = "E_BANG";
                const cloned = structuredClone(original);

                const channel = new MessageChannel();
                const delivered = await new Promise((resolve) => {
                    channel.port2.onmessage = (event) => {
                        resolve({
                            isError: event.data instanceof Error,
                            name: event.data.name,
                            message: event.data.message,
                            code: event.data.code,
                            stackType: typeof event.data.stack,
                        });
                    };

                    channel.port1.postMessage(original);
                });

                return {
                    cloneIsError: cloned instanceof Error,
                    cloneName: cloned.name,
                    cloneMessage: cloned.message,
                    cloneCode: cloned.code,
                    cloneStackType: typeof cloned.stack,
                    delivered,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["cloneIsError"], true);
    assert_eq!(value["cloneName"], "TypeError");
    assert_eq!(value["cloneMessage"], "boom");
    assert_eq!(value["cloneCode"], "E_BANG");
    assert_eq!(value["cloneStackType"], "string");
    assert_eq!(value["delivered"]["isError"], true);
    assert_eq!(value["delivered"]["name"], "TypeError");
    assert_eq!(value["delivered"]["message"], "boom");
    assert_eq!(value["delivered"]["code"], "E_BANG");
    assert_eq!(value["delivered"]["stackType"], "string");
}

#[test]
fn starts_message_ports_and_drains_queued_messages() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const channel = new MessageChannel();
                const received = [];

                channel.port2.addEventListener("message", (event) => {
                    received.push(event.data);
                });

                channel.port1.postMessage("before-start");
                await new Promise((resolve) => setTimeout(resolve, 0));

                const beforeStart = received.slice();
                channel.port2.start();
                await new Promise((resolve) => setTimeout(resolve, 0));

                channel.port1.postMessage("after-start");
                await new Promise((resolve) => setTimeout(resolve, 0));

                return {
                    beforeStart,
                    afterStart: received,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["beforeStart"], serde_json::json!([]));
    assert_eq!(
        value["afterStart"],
        serde_json::json!(["before-start", "after-start"])
    );
}

#[test]
fn reports_worker_startup_errors_as_async_events() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const workerUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "throw new Error('startup boom');",
                            "self.onmessage = () => postMessage('unexpected');",
                        ],
                        { type: "text/javascript" },
                    ),
                );

                let constructorError = "";
                let startupError = null;
                const messages = [];

                try {
                    const worker = new Worker(workerUrl);
                    worker.onmessage = (event) => messages.push(event.data);
                    worker.onerror = (event) => {
                        startupError = {
                            type: event.type,
                            message: event.error?.message ?? "",
                            isError: event.error instanceof Error,
                            targetMatches: event.target === worker,
                        };
                    };
                    worker.postMessage("start");
                    await new Promise((resolve) => setTimeout(resolve, 0));
                } catch (error) {
                    constructorError = String(error.message ?? error);
                }

                return {
                    constructorError,
                    startupError,
                    messages,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["constructorError"], "");
    assert_eq!(value["startupError"]["type"], "error");
    assert_eq!(value["startupError"]["message"], "startup boom");
    assert_eq!(value["startupError"]["isError"], true);
    assert_eq!(value["startupError"]["targetMatches"], true);
    assert_eq!(value["messages"], serde_json::json!([]));
}

#[test]
fn reports_worker_script_load_failures_as_async_events() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
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

        let request = String::from_utf8_lossy(&buffer);
        assert!(request.contains("GET /missing-worker.js HTTP/1.1"));

        let response = concat!(
            "HTTP/1.1 404 Not Found\r\n",
            "Content-Type: text/plain\r\n",
            "Content-Length: 7\r\n",
            "\r\n",
            "missing"
        );

        stream.write_all(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    });

    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let script = format!(
        r#"
        (async () => {{
            let constructorError = "";
            let startupError = null;

            try {{
                const worker = new Worker("http://{address}/missing-worker.js");
                worker.onerror = (event) => {{
                    startupError = {{
                        type: event.type,
                        message: event.error?.message ?? "",
                        isError: event.error instanceof Error,
                        targetMatches: event.target === worker,
                    }};
                }};
                await new Promise((resolve) => setTimeout(resolve, 0));
            }} catch (error) {{
                constructorError = String(error.message ?? error);
            }}

            return {{
                constructorError,
                startupError,
            }};
        }})()
        "#
    );

    let result = runtime.eval_async_as_string(&script).unwrap();
    server.join().unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["constructorError"], "");
    assert_eq!(value["startupError"]["type"], "error");
    assert_eq!(
        value["startupError"]["message"],
        "Failed to construct 'Worker': unable to load script."
    );
    assert_eq!(value["startupError"]["isError"], true);
    assert_eq!(value["startupError"]["targetMatches"], true);
}

#[test]
fn supports_worker_blob_url_and_import_scripts() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const helperUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "self.helperValue = 7;",
                            "self.compute = (value) => value * self.helperValue;",
                        ],
                        { type: "text/javascript" },
                    ),
                );

                const workerUrl = URL.createObjectURL(
                    new Blob(
                        [
                            `importScripts(${JSON.stringify(helperUrl)});`,
                            "self.onmessage = (event) => {",
                            "  const copy = structuredClone(event.data);",
                            "  copy.values.push(self.helperValue);",
                            "  postMessage({",
                            "    kind: event.type,",
                            "    fromWorker: self.compute(copy.base),",
                            "    values: copy.values,",
                            "    nested: copy.nested.value,",
                            "    sameLocation: String(self.location.href).startsWith('blob:'),",
                            "    hasImportScripts: typeof self.importScripts === 'function',",
                            "  });",
                            "};",
                        ].join('\n'),
                        { type: 'text/javascript' },
                    ),
                );

                const worker = new Worker(workerUrl);
                const response = await new Promise((resolve, reject) => {
                    worker.onmessage = (event) => resolve({
                        isMessageEvent: event instanceof MessageEvent,
                        payload: event.data,
                    });
                    worker.onerror = (event) => reject(String(event.error ?? 'worker error'));
                    worker.postMessage({
                        base: 3,
                        values: [1, 2],
                        nested: { value: 5 },
                    });
                });

                return {
                    isMessageEvent: response.isMessageEvent,
                    payload: response.payload,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["isMessageEvent"], true);
    assert_eq!(value["payload"]["kind"], "message");
    assert_eq!(value["payload"]["fromWorker"], 21);
    assert_eq!(value["payload"]["values"], serde_json::json!([1, 2, 7]));
    assert_eq!(value["payload"]["nested"], 5);
    assert_eq!(value["payload"]["sameLocation"], true);
    assert_eq!(value["payload"]["hasImportScripts"], true);
}

#[test]
fn supports_broadcast_channel_delivery() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const first = new BroadcastChannel("rom-sync");
                const second = new BroadcastChannel("rom-sync");
                const third = new BroadcastChannel("rom-sync");
                const received = [];

                second.onmessage = (event) => {
                    received.push({
                        channel: "second",
                        value: event.data.value,
                        isMessageEvent: event instanceof MessageEvent,
                    });
                };
                third.addEventListener("message", (event) => {
                    received.push({
                        channel: "third",
                        value: event.data.value,
                        isMessageEvent: event instanceof MessageEvent,
                    });
                });

                first.postMessage({ value: 4 });
                await new Promise((resolve) => setTimeout(resolve, 0));
                third.close();
                first.postMessage({ value: 9 });
                await new Promise((resolve) => setTimeout(resolve, 0));

                return {
                    received,
                    firstName: first.name,
                    secondName: second.name,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["firstName"], "rom-sync");
    assert_eq!(value["secondName"], "rom-sync");
    assert_eq!(
        value["received"],
        serde_json::json!([
            { "channel": "second", "value": 4, "isMessageEvent": true },
            { "channel": "third", "value": 4, "isMessageEvent": true },
            { "channel": "second", "value": 9, "isMessageEvent": true }
        ])
    );
}

#[test]
fn isolates_worker_timers_from_terminated_scopes() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const terminateUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "self.onmessage = () => {",
                            "  setTimeout(() => postMessage('late-from-terminate'), 20);",
                            "};",
                        ],
                        { type: "text/javascript" },
                    ),
                );

                const closeUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "setTimeout(() => postMessage('late-from-close'), 20);",
                            "close();",
                        ],
                        { type: "text/javascript" },
                    ),
                );

                const terminateWorker = new Worker(terminateUrl);
                const closeWorker = new Worker(closeUrl);
                const events = [];

                terminateWorker.onmessage = (event) => events.push(event.data);
                closeWorker.onmessage = (event) => events.push(event.data);

                terminateWorker.postMessage("start");
                terminateWorker.terminate();

                await new Promise((resolve) => setTimeout(resolve, 60));
                return events;
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value, serde_json::json!([]));
}

#[test]
fn rejects_transfer_lists_for_current_structured_clone_and_messaging_model() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const channel = new MessageChannel();
                const workerUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "self.onmessage = () => {",
                            "  postMessage('unexpected');",
                            "};",
                        ],
                        { type: "text/javascript" },
                    ),
                );
                const worker = new Worker(workerUrl);
                const buffer = new Uint8Array([1, 2, 3]).buffer;

                const capture = (fn) => {
                    try {
                        fn();
                        return "ok";
                    } catch (error) {
                        return error.name;
                    }
                };

                return {
                    structuredCloneTransfer: capture(() =>
                        structuredClone({ buffer }, { transfer: [buffer] }),
                    ),
                    messagePortTransfer: capture(() =>
                        channel.port1.postMessage({ buffer }, [buffer]),
                    ),
                    workerTransfer: capture(() =>
                        worker.postMessage({ buffer }, [buffer]),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["structuredCloneTransfer"], "TypeError");
    assert_eq!(value["messagePortTransfer"], "TypeError");
    assert_eq!(value["workerTransfer"], "TypeError");
}
