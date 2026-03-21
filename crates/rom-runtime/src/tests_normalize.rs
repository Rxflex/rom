use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_node_normalize_for_adjacent_and_empty_text() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("div");
                const nested = document.createElement("span");
                nested.append(
                    document.createTextNode("inner"),
                    document.createTextNode(""),
                    document.createTextNode("-text"),
                );
                root.append(
                    document.createTextNode("a"),
                    document.createTextNode(""),
                    document.createTextNode("b"),
                    nested,
                    document.createTextNode(""),
                    document.createTextNode("c"),
                );

                const records = [];
                const observer = new MutationObserver((entries) => {
                    records.push(
                        ...entries.map((entry) => ({
                            type: entry.type,
                            added: Array.from(entry.addedNodes, (node) => node.nodeName),
                            removed: Array.from(entry.removedNodes, (node) => node.nodeName),
                        })),
                    );
                });
                observer.observe(root, { childList: true, subtree: true, characterData: true });

                root.normalize();
                await Promise.resolve();

                return JSON.stringify({
                    rootNodes: Array.from(root.childNodes, (node) => node.nodeName),
                    rootTexts: Array.from(root.childNodes, (node) => node.textContent),
                    nestedNodes: Array.from(nested.childNodes, (node) => node.nodeName),
                    nestedText: nested.textContent,
                    rootText: root.textContent,
                    recordCount: records.length,
                    removedTextNodes: records.filter((entry) => entry.removed.includes("#text")).length,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["rootNodes"],
        serde_json::json!(["#text", "SPAN", "#text"])
    );
    assert_eq!(
        value["rootTexts"],
        serde_json::json!(["ab", "inner-text", "c"])
    );
    assert_eq!(value["nestedNodes"], serde_json::json!(["#text"]));
    assert_eq!(value["nestedText"], "inner-text");
    assert_eq!(value["rootText"], "abinner-textc");
    assert!(value["recordCount"].as_u64().unwrap_or(0) >= 3);
    assert!(value["removedTextNodes"].as_u64().unwrap_or(0) >= 3);
}
