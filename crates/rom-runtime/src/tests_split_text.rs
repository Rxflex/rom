use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_text_split_text_semantics() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                const text = document.createTextNode("abcdef");
                host.appendChild(text);
                document.body.appendChild(host);

                const records = [];
                const observer = new MutationObserver((entries) => {
                    records.push(
                        ...entries.map((entry) => ({
                            type: entry.type,
                            target: entry.target.nodeName,
                            added: Array.from(entry.addedNodes ?? [], (node) => node.textContent),
                            removed: Array.from(entry.removedNodes ?? [], (node) => node.textContent),
                            oldValue: entry.oldValue ?? null,
                        })),
                    );
                });
                observer.observe(host, {
                    childList: true,
                    characterData: true,
                    subtree: true,
                    characterDataOldValue: true,
                });

                const tail = text.splitText(2);
                const detached = document.createTextNode("xyz");
                const detachedTail = detached.splitText(1);

                let errorName = null;
                try {
                    text.splitText(99);
                } catch (error) {
                    errorName = error.name;
                }

                await Promise.resolve();

                return JSON.stringify({
                    headData: text.data,
                    tailData: tail.data,
                    hostTexts: Array.from(host.childNodes, (node) => node.textContent),
                    tailPreviousSibling: tail.previousSibling === text,
                    tailNextSibling: tail.nextSibling === null,
                    tailOwnerDocument: tail.ownerDocument === document,
                    detachedData: detached.data,
                    detachedTailData: detachedTail.data,
                    detachedTailParent: detachedTail.parentNode === null,
                    errorName,
                    records,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["headData"], "ab");
    assert_eq!(value["tailData"], "cdef");
    assert_eq!(value["hostTexts"], serde_json::json!(["ab", "cdef"]));
    assert_eq!(value["tailPreviousSibling"], true);
    assert_eq!(value["tailNextSibling"], true);
    assert_eq!(value["tailOwnerDocument"], true);
    assert_eq!(value["detachedData"], "x");
    assert_eq!(value["detachedTailData"], "yz");
    assert_eq!(value["detachedTailParent"], true);
    assert_eq!(value["errorName"], "IndexSizeError");
    assert_eq!(
        value["records"],
        serde_json::json!([
            {
                "type": "characterData",
                "target": "#text",
                "added": [],
                "removed": [],
                "oldValue": "abcdef"
            },
            {
                "type": "childList",
                "target": "DIV",
                "added": ["cdef"],
                "removed": [],
                "oldValue": null
            }
        ])
    );
}
