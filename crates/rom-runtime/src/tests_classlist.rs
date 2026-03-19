use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_element_classlist_token_operations() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const element = document.createElement("div");
                element.className = "alpha beta";

                const list = element.classList;
                const containsAlpha = list.contains("alpha");
                list.add("gamma", "beta");
                const removedBeta = (() => {
                    list.remove("beta");
                    return !list.contains("beta");
                })();
                const toggledOn = list.toggle("delta");
                const toggledOff = list.toggle("delta", false);
                const replaced = list.replace("gamma", "omega");

                let invalidTokenError = null;
                try {
                    list.add("bad token");
                } catch (error) {
                    invalidTokenError = error.name;
                }

                return JSON.stringify({
                    containsAlpha,
                    removedBeta,
                    toggledOn,
                    toggledOff,
                    replaced,
                    className: element.className,
                    value: list.value,
                    length: list.length,
                    first: list.item(0),
                    second: list.item(1),
                    stringified: String(list),
                    invalidTokenError,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["containsAlpha"], true);
    assert_eq!(value["removedBeta"], true);
    assert_eq!(value["toggledOn"], true);
    assert_eq!(value["toggledOff"], false);
    assert_eq!(value["replaced"], true);
    assert_eq!(value["className"], "alpha omega");
    assert_eq!(value["value"], "alpha omega");
    assert_eq!(value["length"], 2);
    assert_eq!(value["first"], "alpha");
    assert_eq!(value["second"], "omega");
    assert_eq!(value["stringified"], "alpha omega");
    assert_eq!(value["invalidTokenError"], "SyntaxError");
}
