use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_script_element_src_loading() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const source = "window.__romScriptValue = 41;";
                const scriptUrl = URL.createObjectURL(
                    new Blob([source], { type: "text/javascript" }),
                );

                const script = document.createElement("script");
                script.src = scriptUrl;

                const outcome = await new Promise((resolve) => {
                    script.onload = () => resolve({
                        type: "load",
                        value: window.__romScriptValue,
                        src: script.src,
                    });
                    script.onerror = (event) => resolve({
                        type: "error",
                        message: String(event?.error ?? "error"),
                    });
                    document.head.appendChild(script);
                });

                return JSON.stringify(outcome);
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["type"], "load");
    assert_eq!(value["value"], 41);
    assert!(
        value["src"]
            .as_str()
            .unwrap_or_default()
            .starts_with("blob:")
    );
}

#[test]
fn supports_webpack_style_dynamic_script_and_css_chunk_loading() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const scriptChunkUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "self.__LOADABLE_LOADED_CHUNKS__.push([[7858], {",
                            "50820: function(module) { module.exports = { value: 42 }; }",
                            "}]);",
                        ],
                        { type: "text/javascript" },
                    ),
                );
                const styleChunkUrl = URL.createObjectURL(
                    new Blob(["body { --rom-chunk-loaded: 1; }"], { type: "text/css" }),
                );

                const chunkRegistry = {};
                const modules = {};

                function __webpack_require__(id) {
                    const module = { exports: {} };
                    modules[id].call(module.exports, module, module.exports, __webpack_require__);
                    return module.exports;
                }

                __webpack_require__.m = modules;
                __webpack_require__.f = {};
                __webpack_require__.u = (chunkId) => (
                    chunkId === 7858 ? scriptChunkUrl : ""
                );
                __webpack_require__.miniCssF = () => styleChunkUrl;
                __webpack_require__.o = (target, key) =>
                    Object.prototype.hasOwnProperty.call(target, key);
                __webpack_require__.f.j = (chunkId, promises) => {
                    if (chunkRegistry[chunkId] === 0) {
                        return;
                    }
                    promises.push(
                        new Promise((resolve, reject) => {
                            const script = document.createElement("script");
                            script.src = __webpack_require__.u(chunkId);
                            script.onload = () => {
                                chunkRegistry[chunkId] = 0;
                                resolve();
                            };
                            script.onerror = reject;
                            document.head.appendChild(script);
                        }),
                    );
                };
                __webpack_require__.f.miniCss = (chunkId, promises) => {
                    promises.push(
                        new Promise((resolve, reject) => {
                            const link = document.createElement("link");
                            link.rel = "stylesheet";
                            link.href = __webpack_require__.miniCssF(chunkId);
                            link.onload = resolve;
                            link.onerror = reject;
                            document.head.appendChild(link);
                        }),
                    );
                };
                __webpack_require__.e = (chunkId) =>
                    Promise.all(
                        Object.keys(__webpack_require__.f).reduce((promises, key) => {
                            __webpack_require__.f[key](chunkId, promises);
                            return promises;
                        }, []),
                    );

                self.__LOADABLE_LOADED_CHUNKS__ = [];
                self.__LOADABLE_LOADED_CHUNKS__.push = (chunk) => {
                    for (const [id, factory] of Object.entries(chunk[1])) {
                        __webpack_require__.m[id] = factory;
                    }
                    return 0;
                };

                await __webpack_require__.e(7858);
                const loaded = __webpack_require__(50820);

                return JSON.stringify({
                    moduleCount: Object.keys(__webpack_require__.m).length,
                    loadedValue: loaded.value,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["moduleCount"], 1);
    assert_eq!(value["loadedValue"], 42);
}

#[test]
fn patches_webpack_runtime_chunk_and_minicss_loaders() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const chunkUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "(self.__LOADABLE_LOADED_CHUNKS__=self.__LOADABLE_LOADED_CHUNKS__||[]).push([[7858],{",
                            "50820:function(module){module.exports={value:42}}",
                            "}]);",
                        ],
                        { type: "text/javascript" },
                    ),
                );

                const modules = {};

                function __webpack_require__(id) {
                    const module = { exports: {} };
                    modules[id].call(module.exports, module, module.exports, __webpack_require__);
                    return module.exports;
                }

                __webpack_require__.m = modules;
                __webpack_require__.o = (target, key) =>
                    Object.prototype.hasOwnProperty.call(target, key);
                __webpack_require__.f = {};
                __webpack_require__.u = () => chunkUrl;
                __webpack_require__.p = "";
                __webpack_require__.l = () => {
                    throw new Error("loader should be patched");
                };
                __webpack_require__.f.j = (chunkId, promises) => {
                    promises.push(
                        new Promise((resolve, reject) => {
                            __webpack_require__.l(
                                __webpack_require__.p + __webpack_require__.u(chunkId),
                                (event) => {
                                    if (event?.type === "load") {
                                        resolve();
                                        return;
                                    }
                                    reject(event?.error ?? new Error("chunk load error"));
                                },
                                `chunk-${chunkId}`,
                                chunkId,
                            );
                        }),
                    );
                };
                __webpack_require__.f.miniCss = (_chunkId, promises) => {
                    promises.push(
                        Promise.reject(new Error("miniCss should be patched")),
                    );
                };
                __webpack_require__.e = (chunkId) =>
                    Promise.all(
                        Object.keys(__webpack_require__.f).reduce((promises, key) => {
                            __webpack_require__.f[key](chunkId, promises);
                            return promises;
                        }, []),
                    );

                self.__LOADABLE_LOADED_CHUNKS__ = [];
                self.__LOADABLE_LOADED_CHUNKS__.push = (chunk) => {
                    for (const [id, factory] of Object.entries(chunk[1] || {})) {
                        __webpack_require__.m[id] = factory;
                    }
                    if (typeof chunk[2] === "function") {
                        chunk[2](__webpack_require__);
                    }
                    return 0;
                };

                globalThis.__rom_expose_webpack_require();
                await __webpack_require__.e(7858);
                const loaded = __webpack_require__(50820);

                return JSON.stringify({
                    moduleCount: Object.keys(__webpack_require__.m).length,
                    loadedValue: loaded.value,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["moduleCount"], 1);
    assert_eq!(value["loadedValue"], 42);
}
