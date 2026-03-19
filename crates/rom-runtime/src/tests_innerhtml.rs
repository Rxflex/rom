use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_innerhtml_fragment_parsing_and_serialization() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                host.innerHTML = '<span class="greeting">Hello</span><strong data-kind="accent">world</strong>';

                const first = host.firstChild;
                const second = host.lastChild;
                const initial = {
                    childCount: host.childNodes.length,
                    firstTag: first.tagName,
                    firstClass: first.className,
                    firstText: first.textContent,
                    secondTag: second.tagName,
                    secondAttr: second.getAttribute("data-kind"),
                    secondText: second.textContent,
                    serialized: host.innerHTML,
                };

                host.innerHTML = '<em data-state="ready">updated</em>';

                return JSON.stringify({
                    initial,
                    replaced: {
                        childCount: host.childNodes.length,
                        firstTag: host.firstChild.tagName,
                        attr: host.firstChild.getAttribute("data-state"),
                        text: host.textContent,
                        serialized: host.innerHTML,
                    },
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["initial"]["childCount"], 2);
    assert_eq!(value["initial"]["firstTag"], "SPAN");
    assert_eq!(value["initial"]["firstClass"], "greeting");
    assert_eq!(value["initial"]["firstText"], "Hello");
    assert_eq!(value["initial"]["secondTag"], "STRONG");
    assert_eq!(value["initial"]["secondAttr"], "accent");
    assert_eq!(value["initial"]["secondText"], "world");
    assert_eq!(
        value["initial"]["serialized"],
        "<span class=\"greeting\">Hello</span><strong data-kind=\"accent\">world</strong>"
    );

    assert_eq!(value["replaced"]["childCount"], 1);
    assert_eq!(value["replaced"]["firstTag"], "EM");
    assert_eq!(value["replaced"]["attr"], "ready");
    assert_eq!(value["replaced"]["text"], "updated");
    assert_eq!(
        value["replaced"]["serialized"],
        "<em data-state=\"ready\">updated</em>"
    );
}

#[test]
fn supports_outerhtml_replacement_and_serialization() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const host = document.createElement("div");
                const first = document.createElement("span");
                first.id = "first";
                first.textContent = "one";
                const second = document.createElement("span");
                second.id = "second";
                second.textContent = "two";
                host.append(first, second);

                const originalOuter = first.outerHTML;
                first.outerHTML = '<strong id="replacement">done</strong><em data-kind="tail">tail</em>';

                return JSON.stringify({
                    originalOuter,
                    hostChildren: Array.from(host.childNodes, (node) => node.nodeName),
                    hostChildIds: Array.from(host.children, (node) => node.id || node.getAttribute("data-kind")),
                    textContent: host.textContent,
                    replacementOuter: host.firstChild.outerHTML,
                    tailOuter: host.childNodes[1].outerHTML,
                    detachedOuter: (() => {
                        const detached = document.createElement("p");
                        detached.textContent = "standalone";
                        detached.outerHTML = '<section>ignored</section>';
                        return detached.outerHTML;
                    })(),
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["originalOuter"], "<span id=\"first\">one</span>");
    assert_eq!(
        value["hostChildren"],
        serde_json::json!(["STRONG", "EM", "SPAN"])
    );
    assert_eq!(
        value["hostChildIds"],
        serde_json::json!(["replacement", "tail", "second"])
    );
    assert_eq!(value["textContent"], "donetailtwo");
    assert_eq!(
        value["replacementOuter"],
        "<strong id=\"replacement\">done</strong>"
    );
    assert_eq!(value["tailOuter"], "<em data-kind=\"tail\">tail</em>");
    assert_eq!(value["detachedOuter"], "<p>standalone</p>");
}
