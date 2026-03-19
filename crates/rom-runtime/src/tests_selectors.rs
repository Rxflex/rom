use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_compound_attribute_selectors() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const root = document.createElement("section");

                const primary = document.createElement("div");
                primary.id = "hero";
                primary.className = "card selected";
                primary.setAttribute("data-kind", "primary");
                primary.setAttribute("data-state", "ready");

                const secondary = document.createElement("div");
                secondary.className = "card";
                secondary.setAttribute("data-kind", "secondary");

                root.appendChild(primary);
                root.appendChild(secondary);
                document.body.appendChild(root);

                const nested = document.createElement("span");
                nested.className = "pill";
                secondary.appendChild(nested);

                const checks = {
                    tagAndClass: root.querySelector("div.card") === primary,
                    idAndClass: root.querySelector("#hero.selected") === primary,
                    attrPresence: root.querySelector("[data-kind]") === primary,
                    attrValue: root.querySelector("[data-kind=primary]") === primary,
                    quotedAttrValue: root.querySelector("[data-state=\"ready\"]") === primary,
                    compound: root.querySelector("div.card[data-kind=secondary]") === secondary,
                    matchesClass: primary.matches(".selected"),
                    matchesCompound: secondary.matches("div.card[data-kind=secondary]"),
                    matchesUnsupported: secondary.matches("section div") === false,
                    closestSelf: secondary.closest(".card") === secondary,
                    closestAncestor: nested.closest("[data-kind]") === secondary,
                    closestMiss: nested.closest(".missing") === null,
                    allCards: root.querySelectorAll("div.card").length,
                    selectedCards: root.querySelectorAll(".selected").length,
                    attrMatches: root.querySelectorAll("[data-kind]").length,
                    unsupportedDescendant: root.querySelector("section div") === null,
                };

                return JSON.stringify(checks);
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["tagAndClass"], true);
    assert_eq!(value["idAndClass"], true);
    assert_eq!(value["attrPresence"], true);
    assert_eq!(value["attrValue"], true);
    assert_eq!(value["quotedAttrValue"], true);
    assert_eq!(value["compound"], true);
    assert_eq!(value["matchesClass"], true);
    assert_eq!(value["matchesCompound"], true);
    assert_eq!(value["matchesUnsupported"], true);
    assert_eq!(value["closestSelf"], true);
    assert_eq!(value["closestAncestor"], true);
    assert_eq!(value["closestMiss"], true);
    assert_eq!(value["allCards"], 2);
    assert_eq!(value["selectedCards"], 1);
    assert_eq!(value["attrMatches"], 2);
    assert_eq!(value["unsupportedDescendant"], true);
}
