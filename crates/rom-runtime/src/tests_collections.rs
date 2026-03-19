use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_dom_collection_helpers() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("section");
                const card = document.createElement("div");
                card.className = "card selected";
                const pill = document.createElement("span");
                pill.className = "pill selected";
                const plain = document.createElement("p");
                plain.className = "copy";

                card.appendChild(pill);
                root.append(card, plain);
                document.body.appendChild(root);

                const fragment = document.createDocumentFragment();
                const fragmentItem = document.createElement("li");
                fragmentItem.className = "selected";
                fragment.appendChild(fragmentItem);

                return JSON.stringify({
                    rootDivCount: root.getElementsByTagName("div").length,
                    rootAnyCount: root.getElementsByTagName("*").length,
                    rootSelectedCount: root.getElementsByClassName("selected").length,
                    rootCardSelectedCount: root.getElementsByClassName("card selected").length,
                    documentSpanCount: document.getElementsByTagName("span").length,
                    documentSelectedCount: document.getElementsByClassName("selected").length,
                    fragmentSelectedCount: fragment.getElementsByClassName("selected").length,
                    fragmentTagCount: fragment.getElementsByTagName("li").length,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["rootDivCount"], 1);
    assert_eq!(value["rootAnyCount"], 3);
    assert_eq!(value["rootSelectedCount"], 2);
    assert_eq!(value["rootCardSelectedCount"], 1);
    assert_eq!(value["documentSpanCount"], 1);
    assert_eq!(value["documentSelectedCount"], 2);
    assert_eq!(value["fragmentSelectedCount"], 1);
    assert_eq!(value["fragmentTagCount"], 1);
}
