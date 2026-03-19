use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_node_identity_and_structural_equality() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const left = document.createElement("div");
                left.id = "card";
                left.setAttribute("data-kind", "primary");
                left.style.color = "red";
                left.append(
                    document.createTextNode("hello"),
                    document.createComment("note"),
                    document.createElement("span"),
                );
                left.lastChild.textContent = "world";

                const equalClone = left.cloneNode(true);
                const differentAttr = left.cloneNode(true);
                differentAttr.setAttribute("data-kind", "secondary");

                const differentComment = left.cloneNode(true);
                differentComment.childNodes[1].nodeValue = "changed";

                const differentStyle = left.cloneNode(true);
                differentStyle.style.color = "blue";

                return JSON.stringify({
                    sameSelf: left.isSameNode(left),
                    sameClone: left.isSameNode(equalClone),
                    equalClone: left.isEqualNode(equalClone),
                    equalNull: left.isEqualNode(null),
                    differentAttr: left.isEqualNode(differentAttr),
                    differentComment: left.isEqualNode(differentComment),
                    differentStyle: left.isEqualNode(differentStyle),
                    commentEquality: left.childNodes[1].isEqualNode(document.createComment("note")),
                    textEquality: left.firstChild.isEqualNode(document.createTextNode("hello")),
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["sameSelf"], true);
    assert_eq!(value["sameClone"], false);
    assert_eq!(value["equalClone"], true);
    assert_eq!(value["equalNull"], false);
    assert_eq!(value["differentAttr"], false);
    assert_eq!(value["differentComment"], false);
    assert_eq!(value["differentStyle"], false);
    assert_eq!(value["commentEquality"], true);
    assert_eq!(value["textEquality"], true);
}
