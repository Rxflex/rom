use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_node_document_and_connection_helpers() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                const child = document.createElement("span");
                const text = document.createTextNode("hello");
                child.appendChild(text);

                const detached = document.createElement("em");
                const fragment = document.createDocumentFragment();
                const fragmentChild = document.createElement("strong");
                fragment.appendChild(fragmentChild);

                document.body.appendChild(host);
                host.appendChild(child);

                const beforeDetach = {
                    childOwnerDocument: child.ownerDocument === document,
                    textOwnerDocument: text.ownerDocument === document,
                    fragmentOwnerDocument: fragment.ownerDocument === document,
                    fragmentChildOwnerDocument: fragmentChild.ownerDocument === document,
                    documentBaseURI: document.baseURI,
                    childBaseURI: child.baseURI,
                    textBaseURI: text.baseURI,
                    fragmentBaseURI: fragment.baseURI,
                    detachedBaseURI: detached.baseURI,
                    childRootNode: child.getRootNode() === document,
                    textRootNode: text.getRootNode({ composed: true }) === document,
                    fragmentRootNode: fragment.getRootNode() === fragment,
                    fragmentChildRootNode: fragmentChild.getRootNode() === fragment,
                    detachedRootNode: detached.getRootNode() === detached,
                    childIsConnected: child.isConnected,
                    textIsConnected: text.isConnected,
                    detachedIsConnected: detached.isConnected,
                    documentIsConnected: document.isConnected,
                    hostHasChildren: host.hasChildNodes(),
                    detachedHasChildren: detached.hasChildNodes(),
                    elementNodeValue: child.nodeValue === null,
                    textNodeValue: text.nodeValue,
                };

                text.nodeValue = "updated";
                history.pushState({ step: "base-uri" }, "", "/moved?query=1#hash");
                host.remove();

                return JSON.stringify({
                    beforeDetach,
                    afterNavigationBaseURI: child.baseURI,
                    updatedText: text.textContent,
                    childConnectedAfterRemove: child.isConnected,
                    textConnectedAfterRemove: text.isConnected,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["beforeDetach"]["childOwnerDocument"], true);
    assert_eq!(value["beforeDetach"]["textOwnerDocument"], true);
    assert_eq!(value["beforeDetach"]["fragmentOwnerDocument"], true);
    assert_eq!(value["beforeDetach"]["fragmentChildOwnerDocument"], true);
    assert_eq!(
        value["beforeDetach"]["documentBaseURI"],
        "https://rom.local/"
    );
    assert_eq!(value["beforeDetach"]["childBaseURI"], "https://rom.local/");
    assert_eq!(value["beforeDetach"]["textBaseURI"], "https://rom.local/");
    assert_eq!(
        value["beforeDetach"]["fragmentBaseURI"],
        "https://rom.local/"
    );
    assert_eq!(
        value["beforeDetach"]["detachedBaseURI"],
        "https://rom.local/"
    );
    assert_eq!(value["beforeDetach"]["childRootNode"], true);
    assert_eq!(value["beforeDetach"]["textRootNode"], true);
    assert_eq!(value["beforeDetach"]["fragmentRootNode"], true);
    assert_eq!(value["beforeDetach"]["fragmentChildRootNode"], true);
    assert_eq!(value["beforeDetach"]["detachedRootNode"], true);
    assert_eq!(value["beforeDetach"]["childIsConnected"], true);
    assert_eq!(value["beforeDetach"]["textIsConnected"], true);
    assert_eq!(value["beforeDetach"]["detachedIsConnected"], false);
    assert_eq!(value["beforeDetach"]["documentIsConnected"], true);
    assert_eq!(value["beforeDetach"]["hostHasChildren"], true);
    assert_eq!(value["beforeDetach"]["detachedHasChildren"], false);
    assert_eq!(value["beforeDetach"]["elementNodeValue"], true);
    assert_eq!(value["beforeDetach"]["textNodeValue"], "hello");
    assert_eq!(
        value["afterNavigationBaseURI"],
        "https://rom.local/moved?query=1#hash"
    );
    assert_eq!(value["updatedText"], "updated");
    assert_eq!(value["childConnectedAfterRemove"], false);
    assert_eq!(value["textConnectedAfterRemove"], false);
}
