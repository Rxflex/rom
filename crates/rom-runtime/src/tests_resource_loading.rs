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
fn executes_nested_script_nodes_inserted_via_innerhtml() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.body.innerHTML = `
                    <div class="wrapper">
                        <script>window.__romNestedScriptValue = 42;</script>
                    </div>
                `;

                await new Promise((resolve) => setTimeout(resolve, 0));

                return JSON.stringify({
                    value: window.__romNestedScriptValue,
                    scripts: document.scripts.length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["value"], 42);
    assert_eq!(value["scripts"], 1);
}

#[test]
fn executes_inline_scripts_in_parser_order_during_innerhtml_seeding() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                delete window.__romParserOrder;

                document.head.innerHTML = `
                    <script>
                        window.__romParserOrder = ["first"];
                        const second = document.getElementById("second-inline");
                        if (second) {
                            second.remove();
                        }
                    </script>
                    <script id="second-inline">
                        window.__romParserOrder.push("second");
                    </script>
                `;

                return JSON.stringify({
                    order: window.__romParserOrder,
                    scripts: document.scripts.length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["order"], serde_json::json!(["first", "second"]));
    assert_eq!(value["scripts"], 2);
}

#[test]
fn preserves_script_text_with_less_than_tokens_during_innerhtml_seeding() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.head.innerHTML = `
                    <script>
                        window.__romLessThanOk = 1;
                        for (let index = 0; index < 2; index += 1) {
                            if (index < 10) {
                                window.__romLessThanOk += index;
                            }
                        }
                    </script>
                    <script>window.__romAfterLessThan = 7;</script>
                `;

                return JSON.stringify({
                    firstHasScriptClose: document.scripts[0].textContent.includes("</script>"),
                    firstValue: window.__romLessThanOk,
                    secondValue: window.__romAfterLessThan,
                    count: document.scripts.length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["firstHasScriptClose"], false);
    assert_eq!(value["firstValue"], 2);
    assert_eq!(value["secondValue"], 7);
    assert_eq!(value["count"], 2);
}

#[test]
fn loads_external_scripts_inserted_by_inline_seed_scripts() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const source = "window.__romSeededExternalValue = 73;";
                const scriptUrl = URL.createObjectURL(
                    new Blob([source], { type: "text/javascript" }),
                );

                document.head.innerHTML = `
                    <script>
                        (window.requestAnimationFrame || setTimeout)(function() {
                            const fragment = document.createDocumentFragment();
                            const script = document.createElement("script");
                            script.src = ${JSON.stringify("${scriptUrl}")};
                            fragment.appendChild(script);
                            document.head.appendChild(fragment);
                        });
                    </script>
                `.replace("${scriptUrl}", scriptUrl);

                await new Promise((resolve) => setTimeout(resolve, 20));
                await new Promise((resolve) => setTimeout(resolve, 20));

                return JSON.stringify({
                    value: window.__romSeededExternalValue,
                    scriptCount: document.scripts.length,
                    srcCount: document.scripts.filter((script) => !!script.src).length,
                    srcStarted: document.scripts.filter((script) => !!script.src && !!script.__romLoadStarted).length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["value"], 73);
    assert_eq!(value["scriptCount"], 2);
    assert_eq!(value["srcCount"], 1);
    assert_eq!(value["srcStarted"], 1);
}

#[test]
fn ignores_non_javascript_script_types() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                globalThis.__romJsonLdExecuted = false;
                const script = document.createElement("script");
                script.type = "application/ld+json";
                script.text = "{\"name\":\"Temu\"}";

                await new Promise((resolve, reject) => {
                    script.onload = () => resolve();
                    script.onerror = reject;
                    document.head.appendChild(script);
                });

                return JSON.stringify({
                    executed: globalThis.__romJsonLdExecuted,
                    scriptCount: document.scripts.length,
                    text: script.text,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["executed"], false);
    assert_eq!(value["scriptCount"], 1);
    assert_eq!(value["text"], "{\"name\":\"Temu\"}");
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

#[test]
fn replays_dom_content_loaded_after_inline_script_completion() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const scriptUrl = URL.createObjectURL(
                    new Blob(["window.__romDomReadyScriptValue = 91;"], { type: "text/javascript" }),
                );

                document.head.innerHTML = `
                    <script>
                        document.addEventListener("DOMContentLoaded", function() {
                            const script = document.createElement("script");
                            script.src = "${scriptUrl}";
                            document.head.appendChild(script);
                        });
                    </script>
                `.replace("${scriptUrl}", scriptUrl);

                await new Promise((resolve) => setTimeout(resolve, 20));
                await new Promise((resolve) => setTimeout(resolve, 20));

                return JSON.stringify({
                    value: window.__romDomReadyScriptValue,
                    srcCount: document.scripts.filter((script) => !!script.src).length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["value"], 91);
    assert!(value["srcCount"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn dispatches_dom_content_loaded_after_head_html_seeding() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const firstUrl = URL.createObjectURL(
                    new Blob(["window.__romHeadSeedOne = 1;"], { type: "text/javascript" }),
                );
                const secondUrl = URL.createObjectURL(
                    new Blob(["window.__romHeadSeedTwo = 2;"], { type: "text/javascript" }),
                );

                document.head.innerHTML = `
                    <script>
                        (function() {
                            function start() {
                                const fragment = document.createDocumentFragment();
                                const urls = ["${firstUrl}", "${secondUrl}"];
                                for (let index = 0; index < urls.length; index += 1) {
                                    const script = document.createElement("script");
                                    script.src = urls[index];
                                    script.crossOrigin = "anonymous";
                                    script.async = false;
                                    fragment.appendChild(script);
                                }
                                document.head.appendChild(fragment);
                            }

                            document.addEventListener("DOMContentLoaded", start);
                        })();
                    </script>
                `.replace("${firstUrl}", firstUrl).replace("${secondUrl}", secondUrl);

                await new Promise((resolve) => setTimeout(resolve, 50));
                await new Promise((resolve) => setTimeout(resolve, 50));

                return JSON.stringify({
                    one: window.__romHeadSeedOne,
                    two: window.__romHeadSeedTwo,
                    srcCount: document.scripts.filter((script) => !!script.src).length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["one"], 1);
    assert_eq!(value["two"], 2);
    assert!(value["srcCount"].as_u64().unwrap_or(0) >= 2);
}

#[test]
fn patches_xrender_resource_loader_script_batches() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                window.__XRenderResourcesLoader = {
                    loadScript() {
                        throw new Error("site loader should be patched");
                    },
                    loadScripts(urls, integrities, immediate) {
                        return urls.map((url, index) =>
                            this.loadScript(url, integrities?.[index], immediate)
                        );
                    },
                };

                const scriptUrl = URL.createObjectURL(
                    new Blob(["window.__romXRenderPatched = 33;"], { type: "text/javascript" }),
                );

                const results = window.__XRenderResourcesLoader.loadScripts([scriptUrl], [null], true);
                await Promise.all(results);

                return JSON.stringify({
                    value: window.__romXRenderPatched,
                    srcCount: document.scripts.filter((script) => !!script.src).length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["value"], 33);
    assert_eq!(value["srcCount"], 1);
}

#[test]
fn exposes_document_current_script_for_inline_and_external_scripts() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const inlineScript = document.createElement("script");
                inlineScript.text = `
                    globalThis.__romInlineScriptProbe = {
                        tagName: document.currentScript && document.currentScript.tagName,
                        sameNode: document.currentScript === globalThis.__romInlineScriptNode,
                        src: document.currentScript && document.currentScript.src,
                    };
                `;
                globalThis.__romInlineScriptNode = inlineScript;
                document.head.appendChild(inlineScript);

                await new Promise((resolve) => setTimeout(resolve, 0));

                const externalScriptUrl = URL.createObjectURL(
                    new Blob(
                        [
                            "globalThis.__romExternalScriptProbe = {",
                            "tagName: document.currentScript && document.currentScript.tagName,",
                            "sameNode: document.currentScript === globalThis.__romExternalScriptNode,",
                            "src: document.currentScript && document.currentScript.src,",
                            "scriptsLength: document.scripts.length,",
                            "};",
                        ],
                        { type: "text/javascript" },
                    ),
                );
                const externalScript = document.createElement("script");
                externalScript.src = externalScriptUrl;
                globalThis.__romExternalScriptNode = externalScript;

                await new Promise((resolve, reject) => {
                    externalScript.onload = resolve;
                    externalScript.onerror = reject;
                    document.head.appendChild(externalScript);
                });

                return JSON.stringify({
                    inline: globalThis.__romInlineScriptProbe,
                    external: globalThis.__romExternalScriptProbe,
                    finalCurrentScript: document.currentScript,
                    documentScriptsLength: document.scripts.length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["inline"]["tagName"], "SCRIPT");
    assert_eq!(value["inline"]["sameNode"], true);
    assert_eq!(value["inline"]["src"], "");
    assert_eq!(value["external"]["tagName"], "SCRIPT");
    assert_eq!(value["external"]["sameNode"], true);
    assert!(
        value["external"]["src"]
            .as_str()
            .unwrap_or_default()
            .starts_with("blob:")
    );
    assert_eq!(value["finalCurrentScript"], serde_json::Value::Null);
    assert_eq!(value["documentScriptsLength"], 2);
    assert_eq!(value["external"]["scriptsLength"], 2);
}

#[test]
fn provides_synthetic_current_script_for_nested_eval_inline_scripts() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const before = {
                    present: document.currentScript !== null,
                    tagName: document.currentScript && document.currentScript.tagName,
                    src: document.currentScript && document.currentScript.src,
                };
                const probe = eval(`
                    JSON.stringify({
                        tagName: document.currentScript && document.currentScript.tagName,
                        src: document.currentScript && document.currentScript.src,
                    })
                `);

                return JSON.stringify({
                    before,
                    probe: JSON.parse(probe),
                    after: {
                        present: document.currentScript !== null,
                        tagName: document.currentScript && document.currentScript.tagName,
                        src: document.currentScript && document.currentScript.src,
                    },
                    documentScriptsLength: document.scripts.length,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["before"]["present"], true);
    assert_eq!(value["before"]["tagName"], "SCRIPT");
    assert_eq!(value["probe"]["tagName"], "SCRIPT");
    assert_eq!(value["probe"]["src"], "");
    assert_eq!(value["after"]["present"], true);
    assert_eq!(value["after"]["tagName"], "SCRIPT");
    assert_eq!(value["documentScriptsLength"], 0);
}

#[test]
fn ignores_json_ld_payloads_in_nested_eval_inline_scripts() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const value = eval(`{"@context":"https://schema.org/","@type":"Organization","name":"Temu"}`);
                return JSON.stringify({
                    evalType: typeof value,
                    currentScriptTag: document.currentScript && document.currentScript.tagName,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["evalType"], "undefined");
    assert_eq!(value["currentScriptTag"], "SCRIPT");
}

#[test]
fn reuses_matching_dom_script_nodes_for_nested_eval_context() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                document.body.innerHTML = `
                    <div class="wrapper">
                        <div class="loadable-container" hidden></div>
                        <div class="loadable-styles-before"></div>
                        <div class="loadable-styles-after"><style data-style="1"></style></div>
                        <script>
                            (function() {
                              var s = document.currentScript;
                              var p = s.parentElement;
                              var c = p.querySelector('.loadable-container');
                              var after = p.querySelector('.loadable-styles-after');
                              var before = p.querySelector('.loadable-styles-before');
                              [].slice.call(after.childNodes).forEach(function(c){
                                before.appendChild(c)
                              });
                              c.removeAttribute('hidden');
                            })()
                        </script>
                    </div>
                `;

                const script = document.scripts[0];
                eval(script.textContent);

                const container = document.querySelector(".loadable-container");
                const before = document.querySelector(".loadable-styles-before");
                const after = document.querySelector(".loadable-styles-after");

                return JSON.stringify({
                    hidden: container.getAttribute("hidden"),
                    beforeCount: before.childNodes.length,
                    afterCount: after.childNodes.length,
                    currentScriptTag: document.currentScript && document.currentScript.tagName,
                });
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(value["hidden"], serde_json::Value::Null);
    assert_eq!(value["beforeCount"], 1);
    assert_eq!(value["afterCount"], 0);
    assert_eq!(value["currentScriptTag"], "SCRIPT");
}
