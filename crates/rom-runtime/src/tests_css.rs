use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_css_supports_for_properties_and_conditions() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                return JSON.stringify({
                    propertyPairs: {
                        displayGrid: CSS.supports("display", "grid"),
                        displayNope: CSS.supports("display", "rom-layout"),
                        widthPx: CSS.supports("width", "12px"),
                        widthBad: CSS.supports("width", "wide"),
                        customProperty: CSS.supports("--token", "hotpink"),
                        opacity: CSS.supports("opacity", "0.5"),
                        opacityBad: CSS.supports("opacity", "3"),
                        transform: CSS.supports("transform", "translateX(12px)"),
                    },
                    conditions: {
                        declaration: CSS.supports("(display: flex)"),
                        andCondition: CSS.supports("(display: grid) and (width: 10px)"),
                        orCondition: CSS.supports("(display: rom-layout) or (display: block)"),
                        notCondition: CSS.supports("not (display: rom-layout)"),
                        nested: CSS.supports("((display: grid) and (color: rgb(0, 0, 0)))"),
                        invalidDeclaration: CSS.supports("(display rom-layout)"),
                        invalidCondition: CSS.supports("(display: rom-layout) and (width: nope)"),
                    },
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["propertyPairs"]["displayGrid"], true);
    assert_eq!(value["propertyPairs"]["displayNope"], false);
    assert_eq!(value["propertyPairs"]["widthPx"], true);
    assert_eq!(value["propertyPairs"]["widthBad"], false);
    assert_eq!(value["propertyPairs"]["customProperty"], true);
    assert_eq!(value["propertyPairs"]["opacity"], true);
    assert_eq!(value["propertyPairs"]["opacityBad"], false);
    assert_eq!(value["propertyPairs"]["transform"], true);
    assert_eq!(value["conditions"]["declaration"], true);
    assert_eq!(value["conditions"]["andCondition"], true);
    assert_eq!(value["conditions"]["orCondition"], true);
    assert_eq!(value["conditions"]["notCondition"], true);
    assert_eq!(value["conditions"]["nested"], true);
    assert_eq!(value["conditions"]["invalidDeclaration"], false);
    assert_eq!(value["conditions"]["invalidCondition"], false);
}
