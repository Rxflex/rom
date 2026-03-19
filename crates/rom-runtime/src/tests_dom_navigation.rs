use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_dom_navigation_helpers() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("div");
                const first = document.createElement("span");
                first.id = "first";
                const middleText = document.createTextNode("gap");
                const second = document.createElement("span");
                second.id = "second";
                const third = document.createElement("span");
                third.id = "third";

                root.append(first, middleText, second, third);
                document.body.appendChild(root);

                return JSON.stringify({
                    rootContainsThird: root.contains(third),
                    firstContainsThird: first.contains(third),
                    rootContainsSelf: root.contains(root),
                    firstParentElement: first.parentElement === root,
                    middleParentElement: middleText.parentElement === root,
                    firstNextSibling: first.nextSibling === middleText,
                    textPreviousSibling: middleText.previousSibling === first,
                    textNextSibling: middleText.nextSibling === second,
                    secondPreviousElementSibling: second.previousElementSibling === first,
                    firstNextElementSibling: first.nextElementSibling === second,
                    thirdPreviousElementSibling: third.previousElementSibling === second,
                    rootFirstElementChild: root.firstElementChild === first,
                    rootLastElementChild: root.lastElementChild === third,
                    rootChildElementCount: root.childElementCount,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["rootContainsThird"], true);
    assert_eq!(value["firstContainsThird"], false);
    assert_eq!(value["rootContainsSelf"], true);
    assert_eq!(value["firstParentElement"], true);
    assert_eq!(value["middleParentElement"], true);
    assert_eq!(value["firstNextSibling"], true);
    assert_eq!(value["textPreviousSibling"], true);
    assert_eq!(value["textNextSibling"], true);
    assert_eq!(value["secondPreviousElementSibling"], true);
    assert_eq!(value["firstNextElementSibling"], true);
    assert_eq!(value["thirdPreviousElementSibling"], true);
    assert_eq!(value["rootFirstElementChild"], true);
    assert_eq!(value["rootLastElementChild"], true);
    assert_eq!(value["rootChildElementCount"], 3);
}
