use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_viewport_orientation_and_media_queries() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const list = matchMedia("(min-width: 1200px) and (orientation: landscape)");
                const small = matchMedia("(max-width: 600px)");
                const reducedMotion = matchMedia("(prefers-reduced-motion: no-preference)");
                const events = [];

                list.addEventListener("change", (event) => {
                    events.push(`listener:${event.type}`);
                });
                list.onchange = (event) => {
                    events.push(`handler:${event.type}`);
                };
                list.dispatchEvent(new Event("change"));

                return {
                    innerWidth,
                    innerHeight,
                    outerWidth,
                    outerHeight,
                    devicePixelRatio,
                    visualViewport: {
                        width: visualViewport.width,
                        height: visualViewport.height,
                        scale: visualViewport.scale,
                        isViewport: visualViewport instanceof VisualViewport,
                    },
                    screen: {
                        width: screen.width,
                        height: screen.height,
                        availHeight: screen.availHeight,
                        orientationType: screen.orientation.type,
                        orientationAngle: screen.orientation.angle,
                    },
                    mediaQueries: {
                        listMatches: list.matches,
                        listMedia: list.media,
                        listIsQueryList: list instanceof MediaQueryList,
                        smallMatches: small.matches,
                        reducedMotionMatches: reducedMotion.matches,
                    },
                    events,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["innerWidth"], 1920);
    assert_eq!(value["innerHeight"], 1040);
    assert_eq!(value["outerWidth"], 1920);
    assert_eq!(value["outerHeight"], 1080);
    assert_eq!(value["devicePixelRatio"], 1);
    assert_eq!(value["visualViewport"]["width"], 1920);
    assert_eq!(value["visualViewport"]["height"], 1040);
    assert_eq!(value["visualViewport"]["scale"], 1);
    assert_eq!(value["visualViewport"]["isViewport"], true);
    assert_eq!(value["screen"]["width"], 1920);
    assert_eq!(value["screen"]["height"], 1080);
    assert_eq!(value["screen"]["availHeight"], 1040);
    assert_eq!(value["screen"]["orientationType"], "landscape-primary");
    assert_eq!(value["screen"]["orientationAngle"], 0);
    assert_eq!(value["mediaQueries"]["listMatches"], true);
    assert_eq!(
        value["mediaQueries"]["listMedia"],
        "(min-width: 1200px) and (orientation: landscape)"
    );
    assert_eq!(value["mediaQueries"]["listIsQueryList"], true);
    assert_eq!(value["mediaQueries"]["smallMatches"], false);
    assert_eq!(value["mediaQueries"]["reducedMotionMatches"], true);
    assert_eq!(
        value["events"],
        serde_json::json!(["handler:change", "listener:change"])
    );
}
