use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_history_navigation_and_popstate() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const events = [];
                addEventListener("popstate", (event) => {
                    events.push({
                        href: location.href,
                        state: event.state,
                    });
                });

                history.pushState({ page: 1 }, "", "/page-1?tab=one");
                history.pushState({ page: 2 }, "", "/page-2#details");

                const afterPush = {
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                    length: history.length,
                    state: history.state,
                };

                history.back();
                const afterBack = {
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                    state: history.state,
                };

                history.go(-1);
                const afterGo = {
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                    state: history.state,
                };

                history.forward();
                const afterForward = {
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                    state: history.state,
                };

                history.replaceState({ page: "replaced" }, "", "/page-1?tab=updated");
                const afterReplace = {
                    href: location.href,
                    pathname: location.pathname,
                    search: location.search,
                    hash: location.hash,
                    length: history.length,
                    state: history.state,
                };

                return JSON.stringify({
                    afterPush,
                    afterBack,
                    afterGo,
                    afterForward,
                    afterReplace,
                    events,
                });
            })()
            "#,
        )
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(
        result["afterPush"]["href"],
        "https://rom.local/page-2#details"
    );
    assert_eq!(result["afterPush"]["pathname"], "/page-2");
    assert_eq!(result["afterPush"]["search"], "");
    assert_eq!(result["afterPush"]["hash"], "#details");
    assert_eq!(result["afterPush"]["length"], 3);
    assert_eq!(
        result["afterPush"]["state"],
        serde_json::json!({ "page": 2 })
    );

    assert_eq!(
        result["afterBack"]["href"],
        "https://rom.local/page-1?tab=one"
    );
    assert_eq!(result["afterBack"]["pathname"], "/page-1");
    assert_eq!(result["afterBack"]["search"], "?tab=one");
    assert_eq!(result["afterBack"]["hash"], "");
    assert_eq!(
        result["afterBack"]["state"],
        serde_json::json!({ "page": 1 })
    );

    assert_eq!(result["afterGo"]["href"], "https://rom.local/");
    assert_eq!(result["afterGo"]["pathname"], "/");
    assert_eq!(result["afterGo"]["search"], "");
    assert_eq!(result["afterGo"]["hash"], "");
    assert_eq!(result["afterGo"]["state"], serde_json::Value::Null);

    assert_eq!(
        result["afterForward"]["href"],
        "https://rom.local/page-1?tab=one"
    );
    assert_eq!(result["afterForward"]["pathname"], "/page-1");
    assert_eq!(result["afterForward"]["search"], "?tab=one");
    assert_eq!(result["afterForward"]["hash"], "");
    assert_eq!(
        result["afterForward"]["state"],
        serde_json::json!({ "page": 1 })
    );

    assert_eq!(
        result["afterReplace"]["href"],
        "https://rom.local/page-1?tab=updated"
    );
    assert_eq!(result["afterReplace"]["pathname"], "/page-1");
    assert_eq!(result["afterReplace"]["search"], "?tab=updated");
    assert_eq!(result["afterReplace"]["hash"], "");
    assert_eq!(result["afterReplace"]["length"], 3);
    assert_eq!(
        result["afterReplace"]["state"],
        serde_json::json!({ "page": "replaced" })
    );

    assert_eq!(
        result["events"],
        serde_json::json!([
            {
                "href": "https://rom.local/page-1?tab=one",
                "state": { "page": 1 }
            },
            {
                "href": "https://rom.local/",
                "state": null
            },
            {
                "href": "https://rom.local/page-1?tab=one",
                "state": { "page": 1 }
            }
        ])
    );
}

#[test]
fn validates_history_same_origin_and_location_navigation_entries() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                let crossOriginError = null;
                try {
                    history.pushState({ blocked: true }, "", "https://evil.test/outside");
                } catch (error) {
                    crossOriginError = error.name;
                }

                location.assign("/assigned");
                const afterAssign = {
                    href: location.href,
                    length: history.length,
                    state: history.state,
                };

                history.back();
                const afterAssignBack = {
                    href: location.href,
                    length: history.length,
                    state: history.state,
                };

                location.replace("/replaced");
                const afterReplace = {
                    href: location.href,
                    length: history.length,
                    state: history.state,
                };

                history.forward();
                const afterForward = {
                    href: location.href,
                    length: history.length,
                    state: history.state,
                };

                return JSON.stringify({
                    crossOriginError,
                    afterAssign,
                    afterAssignBack,
                    afterReplace,
                    afterForward,
                });
            })()
            "#,
        )
        .unwrap();
    let result: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(result["crossOriginError"], "SecurityError");
    assert_eq!(result["afterAssign"]["href"], "https://rom.local/assigned");
    assert_eq!(result["afterAssign"]["length"], 2);
    assert_eq!(result["afterAssign"]["state"], serde_json::Value::Null);

    assert_eq!(result["afterAssignBack"]["href"], "https://rom.local/");
    assert_eq!(result["afterAssignBack"]["length"], 2);
    assert_eq!(result["afterAssignBack"]["state"], serde_json::Value::Null);

    assert_eq!(result["afterReplace"]["href"], "https://rom.local/replaced");
    assert_eq!(result["afterReplace"]["length"], 2);
    assert_eq!(result["afterReplace"]["state"], serde_json::Value::Null);

    assert_eq!(result["afterForward"]["href"], "https://rom.local/assigned");
    assert_eq!(result["afterForward"]["length"], 2);
    assert_eq!(result["afterForward"]["state"], serde_json::Value::Null);
}

#[test]
fn dispatches_hashchange_for_location_navigation_and_history_traversal() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r##"
            (async () => {
                const events = [];
                addEventListener("hashchange", (event) => {
                    events.push({
                        type: event.type,
                        oldURL: event.oldURL,
                        newURL: event.newURL,
                        isHashChangeEvent: event instanceof HashChangeEvent,
                    });
                });

                history.pushState({ step: 1 }, "", "/page#push");
                location.assign("#assigned");
                history.back();
                history.forward();
                history.replaceState({ step: 2 }, "", "/page#replaced");

                return JSON.stringify({
                    href: location.href,
                    events,
                });
            })()
            "##,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["href"], "https://rom.local/page#replaced");
    assert_eq!(
        value["events"],
        serde_json::json!([
            {
                "type": "hashchange",
                "oldURL": "https://rom.local/page#push",
                "newURL": "https://rom.local/page#assigned",
                "isHashChangeEvent": true
            },
            {
                "type": "hashchange",
                "oldURL": "https://rom.local/page#assigned",
                "newURL": "https://rom.local/page#push",
                "isHashChangeEvent": true
            },
            {
                "type": "hashchange",
                "oldURL": "https://rom.local/page#push",
                "newURL": "https://rom.local/page#assigned",
                "isHashChangeEvent": true
            }
        ])
    );
}
