use crate::{RomRuntime, RuntimeConfig};

#[test]
fn replays_completed_lifecycle_events_for_late_listeners() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const seen = [];

                document.addEventListener("readystatechange", () => {
                    seen.push(`ready:${document.readyState}`);
                });
                document.addEventListener("DOMContentLoaded", () => {
                    seen.push("dom");
                });
                window.addEventListener("load", () => {
                    seen.push("load");
                });
                window.addEventListener("pageshow", (event) => {
                    seen.push(`pageshow:${event.persisted === false}`);
                });

                await Promise.resolve();
                await Promise.resolve();

                return JSON.stringify({
                    readyState: document.readyState,
                    seen,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["readyState"], "complete");
    assert!(
        value["seen"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == "ready:complete")
    );
    assert!(
        value["seen"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == "dom")
    );
    assert!(
        value["seen"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == "load")
    );
    assert!(
        value["seen"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry == "pageshow:true")
    );
}

#[test]
fn supports_event_listener_objects_for_lifecycle_replays() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                let calls = 0;
                const listener = {
                    handleEvent() {
                        calls += 1;
                    },
                };

                window.addEventListener("load", listener, { once: true });

                await Promise.resolve();
                await Promise.resolve();

                window.dispatchEvent(new Event("load"));

                return JSON.stringify({ calls });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["calls"], 1);
}
