from __future__ import annotations

import json
import time
from typing import Any, Optional


def _build_evaluate_json_script(page_function: Any, arg: Any) -> str:
    if callable(page_function):
        raise TypeError("RomPage.evaluate() in Python accepts a JavaScript string, not a Python callable.")

    return f"""
(async () => {{
    globalThis.__romArg = {json.dumps(None if arg is None else arg)};
    try {{
        const __romValue = await (0, eval)({json.dumps(str(page_function))});
        return JSON.stringify(__romValue === undefined ? null : __romValue);
    }} finally {{
        try {{
            delete globalThis.__romArg;
        }} catch (_error) {{}}
    }}
}})()
""".strip()


def _build_set_content_script(html: str) -> str:
    return f"""
(async () => {{
    const __romHtml = {json.dumps(str(html))};
    const __romDoc = new DOMParser().parseFromString(__romHtml, "text/html");
    const __romRoot = __romDoc.documentElement;
    const __romScriptDescriptors = [];
    let __romNextScriptId = 0;
    for (const __romScript of __romDoc.querySelectorAll("script")) {{
        const __romPlaceholder = __romDoc.createElement("template");
        const __romScriptId = String(__romNextScriptId++);
        __romPlaceholder.setAttribute("data-rom-script-id", __romScriptId);
        __romScript.parentNode.replaceChild(__romPlaceholder, __romScript);
        __romScriptDescriptors.push({{
            id: __romScriptId,
            text: __romScript.text ?? __romScript.textContent ?? "",
            attributes: Array.from(__romScript.attributes.entries()),
        }});
    }}
    document.documentElement.lang = __romRoot?.getAttribute("lang") ?? "";
    document.head.innerHTML = __romDoc.head ? __romDoc.head.innerHTML : "";
    document.body.innerHTML = __romDoc.body ? __romDoc.body.innerHTML : __romHtml;
    for (const __romDescriptor of __romScriptDescriptors) {{
        const __romPlaceholder = document.querySelector('[data-rom-script-id="' + __romDescriptor.id + '"]');
        const __romScript = document.createElement("script");
        for (const [__romName, __romValue] of __romDescriptor.attributes) {{
            __romScript.setAttribute(__romName, __romValue);
        }}
        if (!__romDescriptor.attributes.some(([__romName]) => String(__romName).toLowerCase() === "src")) {{
            __romScript.text = __romDescriptor.text;
        }}
        if (__romPlaceholder?.parentNode) {{
            __romPlaceholder.parentNode.replaceChild(__romScript, __romPlaceholder);
        }}
    }}
    return "ok";
}})()
""".strip()


def _build_goto_script(url: str, options: Optional[dict[str, Any]]) -> str:
    normalized = {
        "url": str(url),
        "method": str((options or {}).get("method") or "GET"),
        "headers": [
            [str(key), str(value)]
            for key, value in ((options or {}).get("headers") or {}).items()
        ],
        "body": (options or {}).get("body"),
    }
    return f"""
(async () => {{
    const __romRequest = {json.dumps(normalized)};
    const __romResponse = await fetch(__romRequest.url, {{
        method: __romRequest.method,
        headers: Object.fromEntries(__romRequest.headers),
        body: __romRequest.body,
    }});
    const __romHtml = await __romResponse.text();
    const __romFinalUrl = __romResponse.url || __romRequest.url;
    const __romParsed = new URL(__romFinalUrl, location.href);
    location.href = __romParsed.href;
    location.origin = __romParsed.origin;
    location.protocol = __romParsed.protocol;
    location.host = __romParsed.host;
    location.hostname = __romParsed.hostname;
    location.pathname = __romParsed.pathname;
    location.search = __romParsed.search;
    location.hash = __romParsed.hash;

    const __romDoc = new DOMParser().parseFromString(__romHtml, "text/html");
    const __romRoot = __romDoc.documentElement;
    const __romScriptDescriptors = [];
    let __romNextScriptId = 0;
    for (const __romScript of __romDoc.querySelectorAll("script")) {{
        const __romPlaceholder = __romDoc.createElement("template");
        const __romScriptId = String(__romNextScriptId++);
        __romPlaceholder.setAttribute("data-rom-script-id", __romScriptId);
        __romScript.parentNode.replaceChild(__romPlaceholder, __romScript);
        __romScriptDescriptors.push({{
            id: __romScriptId,
            text: __romScript.text ?? __romScript.textContent ?? "",
            attributes: Array.from(__romScript.attributes.entries()),
        }});
    }}
    document.documentElement.lang = __romRoot?.getAttribute("lang") ?? "";
    document.head.innerHTML = __romDoc.head ? __romDoc.head.innerHTML : "";
    document.body.innerHTML = __romDoc.body ? __romDoc.body.innerHTML : __romHtml;
    for (const __romDescriptor of __romScriptDescriptors) {{
        const __romPlaceholder = document.querySelector('[data-rom-script-id="' + __romDescriptor.id + '"]');
        const __romScript = document.createElement("script");
        for (const [__romName, __romValue] of __romDescriptor.attributes) {{
            __romScript.setAttribute(__romName, __romValue);
        }}
        if (!__romDescriptor.attributes.some(([__romName]) => String(__romName).toLowerCase() === "src")) {{
            __romScript.text = __romDescriptor.text;
        }}
        if (__romPlaceholder?.parentNode) {{
            __romPlaceholder.parentNode.replaceChild(__romScript, __romPlaceholder);
        }}
    }}

    return JSON.stringify({{
        url: __romFinalUrl,
        status: __romResponse.status,
        ok: __romResponse.ok,
        redirected: Boolean(__romResponse.redirected),
        contentType: __romResponse.headers.get("content-type"),
        bodyLength: __romHtml.length,
    }});
}})()
""".strip()


def _normalize_timeout(options: Optional[dict[str, Any]], default: int = 30_000) -> int:
    try:
        return int((options or {}).get("timeout", default))
    except (TypeError, ValueError):
        return default


def _normalize_polling(options: Optional[dict[str, Any]], default: int = 50) -> int:
    polling = (options or {}).get("polling", default)
    if polling == "raf":
        return 16
    try:
        return int(polling)
    except (TypeError, ValueError):
        return default


def _assert_live_session(runtime: Any) -> None:
    if getattr(runtime, "_native_runtime", None) is not None:
        return

    raise RuntimeError(
        "ROM page helpers require a live native session. CLI bridge mode is stateless across calls."
    )


class RomLocator:
    def __init__(self, page: "RomPage", selector: str) -> None:
        self._page = page
        self._selector = str(selector)

    def click(self, options: Optional[dict[str, Any]] = None) -> None:
        self._page.click(self._selector, options)

    def fill(self, value: Any, options: Optional[dict[str, Any]] = None) -> None:
        self._page.fill(self._selector, value, options)

    def text_content(self, options: Optional[dict[str, Any]] = None) -> Optional[str]:
        return self._page.text_content(self._selector, options)

    def inner_html(self, options: Optional[dict[str, Any]] = None) -> Optional[str]:
        return self._page.inner_html(self._selector, options)

    def wait_for(self, options: Optional[dict[str, Any]] = None) -> Optional["RomLocator"]:
        return self._page.wait_for_selector(self._selector, options)


class RomPage:
    def __init__(self, runtime: Any) -> None:
        self._runtime = runtime

    def locator(self, selector: str) -> RomLocator:
        return RomLocator(self, selector)

    def evaluate(self, page_function: Any, arg: Any = None) -> Any:
        _assert_live_session(self._runtime)
        return self._runtime.eval_json(_build_evaluate_json_script(page_function, arg))

    def content(self) -> str:
        _assert_live_session(self._runtime)
        return self._runtime.eval_async("(async () => document.documentElement.outerHTML)()")

    def set_content(self, html: str, options: Optional[dict[str, Any]] = None) -> None:
        _assert_live_session(self._runtime)
        self._runtime.eval_async(_build_set_content_script(html))
        self.wait_for_load_state((options or {}).get("waitUntil", "load"), _normalize_timeout(options))

    def goto(self, url: str, options: Optional[dict[str, Any]] = None) -> dict[str, Any]:
        _assert_live_session(self._runtime)
        response = self._runtime.eval_json(_build_goto_script(url, options))
        self._runtime.config = {**self._runtime.config, "href": response["url"]}
        self.wait_for_load_state((options or {}).get("waitUntil", "load"), _normalize_timeout(options))
        return response

    def wait_for_load_state(self, wait_until: str = "load", timeout: int = 30_000) -> None:
        normalized = str(wait_until or "load").lower()
        if normalized == "commit":
            return

        if normalized == "domcontentloaded":
            self.wait_for_function(
                "(document.readyState === 'interactive' || document.readyState === 'complete')",
                None,
                {"timeout": timeout, "polling": 25},
            )
            return

        if normalized == "networkidle":
            self.wait_for_function(
                "document.readyState === 'complete'",
                None,
                {"timeout": timeout, "polling": 25},
            )
            time.sleep(0.1)
            return

        self.wait_for_function(
            "document.readyState === 'complete'",
            None,
            {"timeout": timeout, "polling": 25},
        )

    def wait_for_selector(self, selector: str, options: Optional[dict[str, Any]] = None) -> Optional[RomLocator]:
        _assert_live_session(self._runtime)
        timeout = _normalize_timeout(options)
        polling = _normalize_polling(options)
        state = str((options or {}).get("state", "visible"))
        deadline = time.time() + (timeout / 1000.0)

        while time.time() <= deadline:
            status = self.evaluate(
                """
(() => {
    const currentSelector = __romArg.selector;
    const currentState = __romArg.state;
    const element = document.querySelector(currentSelector);
    const isAttached = !!element;
    const isVisible = !!element &&
      element.style?.visibility !== "hidden" &&
      element.offsetWidth > 0 &&
      element.offsetHeight > 0;

    if (currentState === "attached") {
      return { satisfied: isAttached };
    }
    if (currentState === "detached") {
      return { satisfied: !isAttached };
    }
    if (currentState === "hidden") {
      return { satisfied: !isAttached || !isVisible };
    }
    return { satisfied: isVisible };
})()
                """,
                {"selector": str(selector), "state": state},
            )

            if status and status.get("satisfied"):
                if state in {"detached", "hidden"}:
                    return None
                return RomLocator(self, str(selector))

            time.sleep(polling / 1000.0)

        raise RuntimeError(f"Timed out waiting for selector: {selector}")

    def wait_for_function(
        self,
        page_function: Any,
        arg: Any = None,
        options: Optional[dict[str, Any]] = None,
    ) -> Any:
        _assert_live_session(self._runtime)
        timeout = _normalize_timeout(options)
        polling = _normalize_polling(options)
        deadline = time.time() + (timeout / 1000.0)

        while time.time() <= deadline:
            result = self.evaluate(page_function, arg)
            if result:
                return result
            time.sleep(polling / 1000.0)

        raise RuntimeError("Timed out waiting for function.")

    def click(self, selector: str, options: Optional[dict[str, Any]] = None) -> None:
        _assert_live_session(self._runtime)
        self.wait_for_selector(selector, options)
        result = self.evaluate(
            """
(() => {
    const element = document.querySelector(__romArg);
    if (!element) return { ok: false };
    if (typeof element.focus === "function") {
        element.focus();
    }
    for (const type of ["mousedown", "mouseup", "click"]) {
        element.dispatchEvent(new Event(type, { bubbles: true, cancelable: true }));
    }
    return { ok: true };
})()
            """,
            str(selector),
        )
        if not result or not result.get("ok"):
            raise RuntimeError(f"Failed to click selector: {selector}")

    def fill(self, selector: str, value: Any, options: Optional[dict[str, Any]] = None) -> None:
        _assert_live_session(self._runtime)
        self.wait_for_selector(selector, options)
        result = self.evaluate(
            """
(() => {
    const element = document.querySelector(__romArg.selector);
    if (!element) return { ok: false };
    if (typeof element.focus === "function") {
        element.focus();
    }
    const tagName = String(element.tagName ?? "").toUpperCase();
    if ("value" in element || tagName === "INPUT" || tagName === "TEXTAREA" || tagName === "SELECT") {
        element.value = String(__romArg.value);
    } else {
        element.textContent = String(__romArg.value);
    }
    element.dispatchEvent(new Event("input", { bubbles: true, cancelable: true }));
    element.dispatchEvent(new Event("change", { bubbles: true }));
    return { ok: true };
})()
            """,
            {"selector": str(selector), "value": str(value)},
        )
        if not result or not result.get("ok"):
            raise RuntimeError(f"Failed to fill selector: {selector}")

    def text_content(self, selector: str, options: Optional[dict[str, Any]] = None) -> Optional[str]:
        _assert_live_session(self._runtime)
        normalized_options = {**(options or {}), "state": (options or {}).get("state", "attached")}
        self.wait_for_selector(selector, normalized_options)
        return self.evaluate(
            "(() => { const element = document.querySelector(__romArg); return element ? element.textContent : null; })()",
            str(selector),
        )

    def inner_html(self, selector: str, options: Optional[dict[str, Any]] = None) -> Optional[str]:
        _assert_live_session(self._runtime)
        normalized_options = {**(options or {}), "state": (options or {}).get("state", "attached")}
        self.wait_for_selector(selector, normalized_options)
        return self.evaluate(
            "(() => { const element = document.querySelector(__romArg); return element ? element.innerHTML : null; })()",
            str(selector),
        )
