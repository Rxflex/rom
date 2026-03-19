use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_dom_mutation_helper_methods() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("div");
                document.body.appendChild(root);

                const first = document.createElement("span");
                first.id = "first";
                first.textContent = "one";

                const second = document.createElement("span");
                second.id = "second";
                second.textContent = "two";

                root.append(first, second);

                const records = [];
                const observer = new MutationObserver((entries) => {
                    records.push(
                        ...entries.map((entry) => ({
                            added: Array.from(entry.addedNodes, (node) => node.nodeName),
                            removed: Array.from(entry.removedNodes, (node) => node.nodeName),
                        })),
                    );
                });
                observer.observe(root, { childList: true });

                const inserted = document.createElement("i");
                inserted.id = "inserted";
                root.insertBefore(inserted, second);

                const strong = document.createElement("strong");
                strong.id = "after";
                first.after("gap", strong);

                const fragment = document.createDocumentFragment();
                const em = document.createElement("em");
                em.id = "before-second";
                fragment.append(em);
                second.before(fragment);

                const replacement = document.createElement("u");
                replacement.id = "replacement";
                first.replaceWith(replacement);
                second.remove();

                await Promise.resolve();

                return JSON.stringify({
                    nodeNames: Array.from(root.childNodes, (node) => node.nodeName),
                    childIds: Array.from(root.children, (node) => node.id),
                    textContent: root.textContent,
                    records,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["nodeNames"],
        serde_json::json!(["U", "#text", "STRONG", "I", "EM"])
    );
    assert_eq!(
        value["childIds"],
        serde_json::json!(["replacement", "after", "inserted", "before-second"])
    );
    assert_eq!(value["textContent"], "gap");
    assert_eq!(
        value["records"],
        serde_json::json!([
            { "added": ["I"], "removed": [] },
            { "added": ["#text", "STRONG"], "removed": [] },
            { "added": ["EM"], "removed": [] },
            { "added": ["U"], "removed": ["SPAN"] },
            { "added": [], "removed": ["SPAN"] }
        ])
    );
}
