use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_react_dom_host_surface() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const iframe = document.createElement("iframe");
                document.body.appendChild(iframe);

                const input = document.createElement("input");
                document.body.appendChild(input);

                const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
                svg.setAttributeNS(null, "viewBox", "0 0 10 10");
                const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
                circle.setAttributeNS(null, "fill", "red");
                svg.appendChild(circle);

                const beforeFocus = document.activeElement === document.body;
                input.focus();
                const afterFocus = document.activeElement === input;
                input.blur();
                const afterBlur = document.activeElement === document.body;

                return JSON.stringify({
                    hasHtmlIframeElement: typeof HTMLIFrameElement === "function",
                    iframeInstanceof: iframe instanceof HTMLIFrameElement,
                    beforeFocus,
                    afterFocus,
                    afterBlur,
                    hasHasFocus: typeof document.hasFocus === "function",
                    documentHasFocus: document.hasFocus(),
                    svgNamespace: svg.namespaceURI,
                    circleNamespace: circle.namespaceURI,
                    viewBox: svg.getAttribute("viewBox"),
                    fill: circle.getAttribute("fill"),
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["hasHtmlIframeElement"], true);
    assert_eq!(value["iframeInstanceof"], true);
    assert_eq!(value["beforeFocus"], true);
    assert_eq!(value["afterFocus"], true);
    assert_eq!(value["afterBlur"], true);
    assert_eq!(value["hasHasFocus"], true);
    assert_eq!(value["documentHasFocus"], true);
    assert_eq!(value["svgNamespace"], "http://www.w3.org/2000/svg");
    assert_eq!(value["circleNamespace"], "http://www.w3.org/2000/svg");
    assert_eq!(value["viewBox"], "0 0 10 10");
    assert_eq!(value["fill"], "red");
}
