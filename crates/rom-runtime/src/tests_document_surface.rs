use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_document_referrer_and_scroll_offsets() {
    let runtime = RomRuntime::new(RuntimeConfig {
        href: "https://example.test/app".to_owned(),
        referrer: "https://origin.test/landing".to_owned(),
        ..RuntimeConfig::default()
    })
    .unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.documentElement.scrollTop = 128;
                document.documentElement.scrollLeft = 16;

                return JSON.stringify({
                    referrer: document.referrer,
                    hasReferrer: "referrer" in document,
                    scrollTop: document.documentElement.scrollTop,
                    scrollLeft: document.documentElement.scrollLeft,
                    scrollHeight: document.documentElement.scrollHeight,
                    scrollWidth: document.documentElement.scrollWidth,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["referrer"], "https://origin.test/landing");
    assert_eq!(value["hasReferrer"], true);
    assert_eq!(value["scrollTop"], 128);
    assert_eq!(value["scrollLeft"], 16);
    assert!(value["scrollHeight"].as_u64().unwrap_or(0) > 0);
    assert!(value["scrollWidth"].as_u64().unwrap_or(0) > 0);
}
