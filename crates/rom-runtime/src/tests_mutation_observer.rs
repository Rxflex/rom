use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_mutation_observer_child_list_attributes_and_character_data() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const root = document.createElement("section");
                document.body.appendChild(root);

                const records = await new Promise((resolve) => {
                    const observer = new MutationObserver((entries) => {
                        resolve(
                            entries.map((entry) => ({
                                type: entry.type,
                                target: entry.target.nodeName,
                                added: Array.from(entry.addedNodes ?? [], (node) => node.nodeName),
                                removed: Array.from(entry.removedNodes ?? [], (node) => node.nodeName),
                                attributeName: entry.attributeName,
                                oldValue: entry.oldValue,
                            })),
                        );
                    });

                    observer.observe(root, {
                        childList: true,
                        attributes: true,
                        characterData: true,
                        subtree: true,
                        attributeOldValue: true,
                        characterDataOldValue: true,
                    });

                    const child = document.createElement("span");
                    root.appendChild(child);
                    child.setAttribute("data-state", "draft");
                    child.setAttribute("data-state", "final");
                    child.textContent = "hello";
                    child.firstChild.textContent = "world";
                    root.removeChild(child);
                });

                return records;
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value,
        serde_json::json!([
            {
                "type": "childList",
                "target": "SECTION",
                "added": ["SPAN"],
                "removed": [],
                "attributeName": null,
                "oldValue": null
            },
            {
                "type": "attributes",
                "target": "SPAN",
                "added": [],
                "removed": [],
                "attributeName": "data-state",
                "oldValue": null
            },
            {
                "type": "attributes",
                "target": "SPAN",
                "added": [],
                "removed": [],
                "attributeName": "data-state",
                "oldValue": "draft"
            },
            {
                "type": "childList",
                "target": "SPAN",
                "added": ["#text"],
                "removed": [],
                "attributeName": null,
                "oldValue": null
            },
            {
                "type": "characterData",
                "target": "#text",
                "added": [],
                "removed": [],
                "attributeName": null,
                "oldValue": "hello"
            },
            {
                "type": "childList",
                "target": "SECTION",
                "added": [],
                "removed": ["SPAN"],
                "attributeName": null,
                "oldValue": null
            }
        ])
    );
}
