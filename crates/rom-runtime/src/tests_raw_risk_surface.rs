use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_raw_risk_surface_expectations() {
    let runtime = RomRuntime::new(RuntimeConfig {
        href: "https://example.test/path".to_owned(),
        referrer: "https://referrer.example/origin".to_owned(),
        ..RuntimeConfig::default()
    })
    .unwrap();

    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.cookie = "api_uid=seeded; path=/";
                localStorage.setItem("rom-risk", "ready");

                Object.defineProperty(navigator, "userAgent", {
                    configurable: true,
                    get: () => "Mozilla/5.0 Chrome/137.0.0.0",
                });

                let eventCount = 0;
                window.addEventListener("rom-risk-event", () => {
                    eventCount += 1;
                });
                window.dispatchEvent(new Event("rom-risk-event"));

                return JSON.stringify({
                    cookie: document.cookie,
                    localStorage: localStorage.getItem("rom-risk"),
                    userAgent: navigator.userAgent,
                    referrer: document.referrer,
                    scrollTop: document.documentElement.scrollTop,
                    scrollLeft: document.documentElement.scrollLeft,
                    hasDispatchEvent: typeof window.dispatchEvent,
                    eventCount,
                    historyBackLooksElectron: String(history.back).includes("ipcRenderer"),
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["cookie"], "api_uid=seeded");
    assert_eq!(value["localStorage"], "ready");
    assert_eq!(value["userAgent"], "Mozilla/5.0 Chrome/137.0.0.0");
    assert_eq!(value["referrer"], "https://referrer.example/origin");
    assert_eq!(value["scrollTop"], 0);
    assert_eq!(value["scrollLeft"], 0);
    assert_eq!(value["hasDispatchEvent"], "function");
    assert_eq!(value["eventCount"], 1);
    assert_eq!(value["historyBackLooksElectron"], false);
}
