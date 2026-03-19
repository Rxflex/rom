use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_element_dataset_reflection() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const element = document.createElement("div");
                element.setAttribute("data-user-id", "42");
                element.setAttribute("data-state", "draft");

                const initialKeys = Object.keys(element.dataset).sort();
                const hasUserIdBefore = "userId" in element.dataset;
                const initialUserId = element.dataset.userId;

                element.dataset.userId = "84";
                element.dataset.apiToken = null;
                delete element.dataset.state;

                return JSON.stringify({
                    initialKeys,
                    hasUserIdBefore,
                    initialUserId,
                    updatedUserId: element.dataset.userId,
                    userIdAttr: element.getAttribute("data-user-id"),
                    apiTokenValue: element.dataset.apiToken,
                    apiTokenAttr: element.getAttribute("data-api-token"),
                    removedState: element.hasAttribute("data-state"),
                    hasStateAfterDelete: "state" in element.dataset,
                    finalKeys: Object.keys(element.dataset).sort(),
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["initialKeys"], serde_json::json!(["state", "userId"]));
    assert_eq!(value["hasUserIdBefore"], true);
    assert_eq!(value["initialUserId"], "42");
    assert_eq!(value["updatedUserId"], "84");
    assert_eq!(value["userIdAttr"], "84");
    assert_eq!(value["apiTokenValue"], "null");
    assert_eq!(value["apiTokenAttr"], "null");
    assert_eq!(value["removedState"], false);
    assert_eq!(value["hasStateAfterDelete"], false);
    assert_eq!(
        value["finalKeys"],
        serde_json::json!(["apiToken", "userId"])
    );
}
