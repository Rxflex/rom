use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_comment_nodes_and_serialization() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                const text = document.createTextNode("alpha");
                const comment = document.createComment("note");
                const child = document.createElement("span");
                child.textContent = "beta";

                host.append(text, comment, child);
                document.body.appendChild(host);

                const cloned = comment.cloneNode();
                comment.nodeValue = "updated";

                return JSON.stringify({
                    childNodes: Array.from(host.childNodes, (node) => node.nodeName),
                    textContent: host.textContent,
                    innerHTML: host.innerHTML,
                    commentData: comment.data,
                    commentTextContent: comment.textContent,
                    commentNodeValue: comment.nodeValue,
                    commentOwnerDocument: comment.ownerDocument === document,
                    commentBaseURI: comment.baseURI,
                    clonedData: cloned.data,
                    clonedNodeName: cloned.nodeName,
                    clonedNodeValue: cloned.nodeValue,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        value["childNodes"],
        serde_json::json!(["#text", "#comment", "SPAN"])
    );
    assert_eq!(value["textContent"], "alphabeta");
    assert_eq!(value["innerHTML"], "alpha<!--updated--><span>beta</span>");
    assert_eq!(value["commentData"], "updated");
    assert_eq!(value["commentTextContent"], "updated");
    assert_eq!(value["commentNodeValue"], "updated");
    assert_eq!(value["commentOwnerDocument"], true);
    assert_eq!(value["commentBaseURI"], "https://rom.local/");
    assert_eq!(value["clonedData"], "note");
    assert_eq!(value["clonedNodeName"], "#comment");
    assert_eq!(value["clonedNodeValue"], "note");
}
