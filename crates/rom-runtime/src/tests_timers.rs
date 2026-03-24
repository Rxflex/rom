use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_timeout_delay_and_cancellation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const started = performance.now();
                const events = [];

                const cancelled = setTimeout(() => {
                    events.push("cancelled");
                }, 5);
                clearTimeout(cancelled);

                await new Promise((resolve) => {
                    setTimeout(() => {
                        events.push("fired");
                        resolve();
                    }, 30);
                });

                return {
                    events,
                    elapsed: performance.now() - started,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["events"], serde_json::json!(["fired"]));
    assert!(value["elapsed"].as_f64().unwrap() >= 20.0);
}

#[test]
fn supports_interval_repetition_and_clear() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const started = performance.now();
                const ticks = [];

                await new Promise((resolve) => {
                    const intervalId = setInterval(() => {
                        ticks.push(performance.now() - started);

                        if (ticks.length === 3) {
                            clearInterval(intervalId);
                            resolve();
                        }
                    }, 15);
                });

                return {
                    count: ticks.length,
                    firstTick: ticks[0],
                    lastTick: ticks[ticks.length - 1],
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["count"], 3);
    assert!(value["firstTick"].as_f64().unwrap() >= 10.0);
    assert!(value["lastTick"].as_f64().unwrap() >= 35.0);
}

#[test]
fn supports_request_animation_frame_promises_and_microtasks() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const events = [];
                const started = performance.now();

                const frameTime = await new Promise((resolve) => {
                    requestAnimationFrame((timestamp) => {
                        events.push("frame");
                        queueMicrotask(() => {
                            events.push("microtask");
                        });
                        resolve(timestamp);
                    });
                });

                await Promise.resolve();

                return {
                    frameTime,
                    elapsed: performance.now() - started,
                    events,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(value["frameTime"].as_f64().unwrap() >= 0.0);
    assert!(value["elapsed"].as_f64().unwrap() >= 10.0);
    assert_eq!(value["events"], serde_json::json!(["frame", "microtask"]));
}
