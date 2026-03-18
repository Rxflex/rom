use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_event_propagation_and_listener_options() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const root = document.createElement("div");
                const parent = document.createElement("section");
                const child = document.createElement("button");
                root.id = "root";
                parent.id = "parent";
                child.id = "child";
                document.body.appendChild(root);
                root.appendChild(parent);
                parent.appendChild(child);

                const trace = [];
                const removable = () => trace.push("removed");
                const onceListener = () => trace.push("once");
                const stopLog = [];

                root.addEventListener("tap", (event) => {
                    trace.push(`root-capture:${event.eventPhase}`);
                }, { capture: true });
                parent.addEventListener("tap", (event) => {
                    trace.push(`parent-capture:${event.eventPhase}`);
                }, { capture: true, once: true });
                child.addEventListener("tap", onceListener, { once: true });
                child.addEventListener("tap", removable);
                child.removeEventListener("tap", removable);
                child.addEventListener("tap", (event) => {
                    trace.push(`child:${event.eventPhase}`);
                });
                parent.addEventListener("tap", (event) => {
                    trace.push(`parent-bubble:${event.eventPhase}`);
                });
                root.addEventListener("tap", (event) => {
                    trace.push(`root-bubble:${event.eventPhase}`);
                });

                child.addEventListener("halt", (event) => {
                    stopLog.push("first");
                    event.stopImmediatePropagation();
                });
                child.addEventListener("halt", () => {
                    stopLog.push("second");
                });
                root.addEventListener("halt", () => {
                    stopLog.push("root");
                });

                const tapEvent = new Event("tap", { bubbles: true, cancelable: true, composed: true });
                child.dispatchEvent(tapEvent);
                child.dispatchEvent(new Event("tap", { bubbles: true }));
                child.dispatchEvent(new Event("halt", { bubbles: true }));

                return {
                    trace,
                    stopLog,
                    defaultPrevented: tapEvent.defaultPrevented,
                    composedPath: tapEvent.composedPath().map((entry) => entry.id || entry.nodeName),
                    composed: tapEvent.composed,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["trace"],
        serde_json::json!([
            "root-capture:1",
            "parent-capture:1",
            "once",
            "child:2",
            "parent-bubble:3",
            "root-bubble:3",
            "root-capture:1",
            "child:2",
            "parent-bubble:3",
            "root-bubble:3"
        ])
    );
    assert_eq!(value["stopLog"], serde_json::json!(["first"]));
    assert_eq!(value["defaultPrevented"], false);
    assert_eq!(
        value["composedPath"],
        serde_json::json!(["child", "parent", "root", "BODY", "HTML", "#document"])
    );
    assert_eq!(value["composed"], true);
}
