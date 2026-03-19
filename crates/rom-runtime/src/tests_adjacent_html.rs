use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_insert_adjacent_html_text_and_element() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("div");
                const target = document.createElement("span");
                target.id = "target";
                target.textContent = "core";
                root.appendChild(target);
                document.body.appendChild(root);

                target.insertAdjacentHTML("beforebegin", '<b id="before">before</b>');
                target.insertAdjacentText("afterbegin", "start-");
                target.insertAdjacentHTML("beforeend", '<i id="inner">inside</i>');

                const after = document.createElement("u");
                after.id = "after";
                after.textContent = "after";
                const returned = target.insertAdjacentElement("afterend", after);

                const detached = document.createElement("p");
                detached.id = "detached";
                const detachedResult = detached.insertAdjacentElement("beforebegin", document.createElement("em"));

                let invalidPositionError = null;
                try {
                    target.insertAdjacentHTML("sideways", "<x-test></x-test>");
                } catch (error) {
                    invalidPositionError = error.name;
                }

                return JSON.stringify({
                    childNames: Array.from(root.childNodes, (node) => node.nodeName),
                    childIds: Array.from(root.children, (node) => node.id),
                    targetInnerHTML: target.innerHTML,
                    targetText: target.textContent,
                    returnedMatches: returned === after,
                    detachedResult: detachedResult,
                    invalidPositionError,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["childNames"], serde_json::json!(["B", "SPAN", "U"]));
    assert_eq!(
        value["childIds"],
        serde_json::json!(["before", "target", "after"])
    );
    assert_eq!(
        value["targetInnerHTML"],
        "start-core<i id=\"inner\">inside</i>"
    );
    assert_eq!(value["targetText"], "start-coreinside");
    assert_eq!(value["returnedMatches"], true);
    assert_eq!(value["detachedResult"], serde_json::Value::Null);
    assert_eq!(value["invalidPositionError"], "SyntaxError");
}
