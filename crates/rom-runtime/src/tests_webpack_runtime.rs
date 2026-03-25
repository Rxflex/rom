use crate::{RomRuntime, RuntimeConfig};

#[test]
fn exposes_webpack_require_from_loadable_chunk_runtime() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    runtime
        .eval_async_as_string(
            r#"
            (async () => {
                eval(`
                    !function() {
                        "use strict";
                        var c = function webpackRequire() {};
                        c.answer = 42;
                        var n;
                        var loaded = {};
                        var t = function(push, chunk) {
                            var ids = chunk[0];
                            var runtime = chunk[2];
                            if (ids.some(function(id) { return loaded[id] !== 0; })) {
                                if (runtime) {
                                    runtime(c);
                                }
                            }
                            for (var index = 0; index < ids.length; index += 1) {
                                loaded[ids[index]] = 0;
                            }
                            if (typeof push === "function") {
                                push(chunk);
                            }
                            return 0;
                        };
                        n = self.__LOADABLE_LOADED_CHUNKS__ = self.__LOADABLE_LOADED_CHUNKS__ || [];
                        n.forEach(t.bind(null, 0));
                        n.push = t.bind(null, n.push.bind(n));
                    }();
                `);

                return "loaded";
            })()
            "#,
        )
        .unwrap();

    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => JSON.stringify({
                webpackRequire: typeof __webpack_require__,
                windowWebpackRequire: typeof window.__webpack_require__,
                answer: __webpack_require__ && __webpack_require__.answer,
            }))()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["webpackRequire"], "function");
    assert_eq!(value["windowWebpackRequire"], "function");
    assert_eq!(value["answer"], 42);
}

#[test]
fn exposes_promise_rejection_event_for_browser_polyfill_guards() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const env = "BROWSER";
                const polyfillGuard = !(
                    false ||
                    ("BROWSER" !== env && "DENO" !== env) ||
                    typeof PromiseRejectionEvent === "function"
                );

                const promise = Promise.resolve("ok");
                const event = new PromiseRejectionEvent("unhandledrejection", {
                    promise,
                    reason: "boom",
                });

                return JSON.stringify({
                    type: typeof PromiseRejectionEvent,
                    polyfillGuard,
                    eventType: event.type,
                    reason: event.reason,
                    promiseMatches: event.promise === promise,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["type"], "function");
    assert_eq!(value["polyfillGuard"], false);
    assert_eq!(value["eventType"], "unhandledrejection");
    assert_eq!(value["reason"], "boom");
    assert_eq!(value["promiseMatches"], true);
}
