use crate::{RomRuntime, RuntimeConfig};

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
