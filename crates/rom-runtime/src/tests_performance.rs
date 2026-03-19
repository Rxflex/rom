use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_performance_mark_measure_and_entry_queries() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                performance.clearMarks();
                performance.clearMeasures();

                performance.mark("boot", { detail: { phase: 1 } });
                await new Promise((resolve) => setTimeout(resolve, 5));
                performance.mark("middle");
                performance.measure("boot->middle", "boot", "middle");
                performance.measure("window", { start: "boot", end: "middle", detail: { kind: "named" } });
                performance.measure("tail", { start: "middle", duration: 7 });

                const allEntries = performance.getEntries().map((entry) => entry.toJSON());
                const markNames = performance.getEntriesByType("mark").map((entry) => entry.name);
                const namedMeasure = performance.getEntriesByName("boot->middle", "measure")[0]?.toJSON() ?? null;

                performance.clearMarks("middle");
                performance.clearMeasures("tail");

                return JSON.stringify({
                    allEntries,
                    markNames,
                    namedMeasure,
                    remainingMarks: performance.getEntriesByType("mark").map((entry) => entry.name),
                    remainingMeasures: performance.getEntriesByType("measure").map((entry) => entry.name),
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["markNames"], serde_json::json!(["boot", "middle"]));
    assert_eq!(value["allEntries"].as_array().unwrap().len(), 5);
    assert_eq!(value["namedMeasure"]["name"], "boot->middle");
    assert_eq!(value["namedMeasure"]["entryType"], "measure");
    assert!(value["namedMeasure"]["duration"].as_f64().unwrap() >= 0.0);
    assert_eq!(value["remainingMarks"], serde_json::json!(["boot"]));
    assert_eq!(
        value["remainingMeasures"],
        serde_json::json!(["boot->middle", "window"])
    );
}

#[test]
fn supports_performance_observer_and_missing_mark_errors() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                performance.clearMarks();
                performance.clearMeasures();

                const supported = PerformanceObserver.supportedEntryTypes.slice();
                const batches = [];
                const observer = new PerformanceObserver((list) => {
                    batches.push(
                        list.getEntries().map((entry) => ({
                            name: entry.name,
                            type: entry.entryType,
                        })),
                    );
                });

                observer.observe({ entryTypes: ["mark", "measure"] });
                performance.mark("observer-start");
                performance.measure("observer-span", { start: "observer-start" });
                await Promise.resolve();

                let missingMarkError = null;
                try {
                    performance.measure("broken", "missing-mark");
                } catch (error) {
                    missingMarkError = error.name;
                }

                performance.mark("queued");
                const taken = observer.takeRecords().map((entry) => entry.name);
                observer.disconnect();

                return JSON.stringify({
                    supported,
                    batches,
                    missingMarkError,
                    taken,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["supported"], serde_json::json!(["mark", "measure"]));
    assert_eq!(
        value["batches"],
        serde_json::json!([
            [
                { "name": "observer-start", "type": "mark" },
                { "name": "observer-span", "type": "measure" }
            ]
        ])
    );
    assert_eq!(value["missingMarkError"], "SyntaxError");
    assert_eq!(value["taken"], serde_json::json!(["queued"]));
}
