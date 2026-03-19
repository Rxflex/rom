use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_document_fragment_structure_and_append_semantics() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                document.body.appendChild(host);

                const fragment = document.createDocumentFragment();
                const first = document.createElement("span");
                first.id = "first";
                const second = document.createElement("span");
                second.setAttribute("data-kind", "second");
                fragment.append(first, "hello", second);

                const beforeAppend = {
                    isFragment: fragment instanceof DocumentFragment,
                    nodeType: fragment.nodeType,
                    nodeName: fragment.nodeName,
                    childCount: fragment.childNodes.length,
                    firstMatch: fragment.querySelector("#first") === first,
                    secondMatch: fragment.querySelector("[data-kind=second]") === second,
                };

                host.appendChild(fragment);

                return JSON.stringify({
                    beforeAppend,
                    afterAppend: {
                        fragmentChildCount: fragment.childNodes.length,
                        hostChildNodes: Array.from(host.childNodes, (node) => node.nodeName),
                        hostChildren: Array.from(host.children, (node) => node.id || node.getAttribute("data-kind")),
                        textContent: host.textContent,
                        firstParentMatches: first.parentNode === host,
                        secondParentMatches: second.parentNode === host,
                    },
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["beforeAppend"]["isFragment"], true);
    assert_eq!(value["beforeAppend"]["nodeType"], 11);
    assert_eq!(value["beforeAppend"]["nodeName"], "#document-fragment");
    assert_eq!(value["beforeAppend"]["childCount"], 3);
    assert_eq!(value["beforeAppend"]["firstMatch"], true);
    assert_eq!(value["beforeAppend"]["secondMatch"], true);

    assert_eq!(value["afterAppend"]["fragmentChildCount"], 0);
    assert_eq!(
        value["afterAppend"]["hostChildNodes"],
        serde_json::json!(["SPAN", "#text", "SPAN"])
    );
    assert_eq!(
        value["afterAppend"]["hostChildren"],
        serde_json::json!(["first", "second"])
    );
    assert_eq!(value["afterAppend"]["textContent"], "hello");
    assert_eq!(value["afterAppend"]["firstParentMatches"], true);
    assert_eq!(value["afterAppend"]["secondParentMatches"], true);
}
