function serializeArg(value) {
  return JSON.stringify(value === undefined ? null : value);
}

function buildEvaluateJsonScript(pageFunction, arg) {
  const serializedArg = serializeArg(arg);

  if (typeof pageFunction === "function") {
    return `(async () => {
      const __romArg = ${serializedArg};
      const __romValue = await (${pageFunction.toString()})(__romArg);
      return JSON.stringify(__romValue === undefined ? null : __romValue);
    })()`;
  }

  return `(async () => {
    globalThis.__romArg = ${serializedArg};
    try {
      const __romValue = await (0, eval)(${JSON.stringify(String(pageFunction))});
      return JSON.stringify(__romValue === undefined ? null : __romValue);
    } finally {
      try {
        delete globalThis.__romArg;
      } catch {}
    }
  })()`;
}

function buildSetContentScript(html) {
  return `(async () => {
    const __romHtml = ${JSON.stringify(String(html))};
    const __romDoc = new DOMParser().parseFromString(__romHtml, "text/html");
    const __romRoot = __romDoc.documentElement;
    const __romScriptDescriptors = [];
    let __romNextScriptId = 0;
    for (const __romScript of __romDoc.querySelectorAll("script")) {
      const __romPlaceholder = __romDoc.createElement("template");
      const __romScriptId = String(__romNextScriptId++);
      __romPlaceholder.setAttribute("data-rom-script-id", __romScriptId);
      __romScript.parentNode.replaceChild(__romPlaceholder, __romScript);
      __romScriptDescriptors.push({
        id: __romScriptId,
        text: __romScript.text ?? __romScript.textContent ?? "",
        attributes: Array.from(__romScript.attributes.entries()),
      });
    }

    document.documentElement.lang = __romRoot?.getAttribute("lang") ?? "";
    document.head.innerHTML = __romDoc.head ? __romDoc.head.innerHTML : "";
    document.body.innerHTML = __romDoc.body ? __romDoc.body.innerHTML : __romHtml;

    for (const __romDescriptor of __romScriptDescriptors) {
      const __romPlaceholder = document.querySelector(
        '[data-rom-script-id="' + __romDescriptor.id + '"]',
      );
      const __romScript = document.createElement("script");
      for (const [__romName, __romValue] of __romDescriptor.attributes) {
        __romScript.setAttribute(__romName, __romValue);
      }
      if (!__romDescriptor.attributes.some(([__romName]) => String(__romName).toLowerCase() === "src")) {
        __romScript.text = __romDescriptor.text;
      }
      if (__romPlaceholder?.parentNode) {
        __romPlaceholder.parentNode.replaceChild(__romScript, __romPlaceholder);
      }
    }
    return "ok";
  })()`;
}

function buildGotoScript(url, options = {}) {
  const payload = {
    url: String(url),
    method: options.method ? String(options.method) : "GET",
    headers: Object.entries(options.headers ?? {}).map(([key, value]) => [String(key), String(value)]),
    body: options.body ?? null,
  };

  return `(async () => {
    const __romRequest = ${JSON.stringify(payload)};
    const __romResponse = await fetch(__romRequest.url, {
      method: __romRequest.method,
      headers: Object.fromEntries(__romRequest.headers),
      body: __romRequest.body,
    });
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
    for (const __romScript of __romDoc.querySelectorAll("script")) {
      const __romPlaceholder = __romDoc.createElement("template");
      const __romScriptId = String(__romNextScriptId++);
      __romPlaceholder.setAttribute("data-rom-script-id", __romScriptId);
      __romScript.parentNode.replaceChild(__romPlaceholder, __romScript);
      __romScriptDescriptors.push({
        id: __romScriptId,
        text: __romScript.text ?? __romScript.textContent ?? "",
        attributes: Array.from(__romScript.attributes.entries()),
      });
    }

    document.documentElement.lang = __romRoot?.getAttribute("lang") ?? "";
    document.head.innerHTML = __romDoc.head ? __romDoc.head.innerHTML : "";
    document.body.innerHTML = __romDoc.body ? __romDoc.body.innerHTML : __romHtml;

    for (const __romDescriptor of __romScriptDescriptors) {
      const __romPlaceholder = document.querySelector(
        '[data-rom-script-id="' + __romDescriptor.id + '"]',
      );
      const __romScript = document.createElement("script");
      for (const [__romName, __romValue] of __romDescriptor.attributes) {
        __romScript.setAttribute(__romName, __romValue);
      }
      if (!__romDescriptor.attributes.some(([__romName]) => String(__romName).toLowerCase() === "src")) {
        __romScript.text = __romDescriptor.text;
      }
      if (__romPlaceholder?.parentNode) {
        __romPlaceholder.parentNode.replaceChild(__romScript, __romPlaceholder);
      }
    }

    return JSON.stringify({
      url: __romFinalUrl,
      status: __romResponse.status,
      ok: __romResponse.ok,
      redirected: Boolean(__romResponse.redirected),
      contentType: __romResponse.headers.get("content-type"),
      bodyLength: __romHtml.length,
    });
  })()`;
}

async function sleep(delayMs) {
  await new Promise((resolve) => setTimeout(resolve, delayMs));
}

function assertLiveSession(runtime) {
  if (typeof runtime.hasLiveSession === "function" && runtime.hasLiveSession()) {
    return;
  }

  throw new Error(
    "ROM page helpers require a live native session. CLI bridge mode is stateless across calls.",
  );
}

async function waitForLoadState(page, waitUntil = "load", timeout = 30_000) {
  const normalized = String(waitUntil ?? "load").toLowerCase();
  if (normalized === "commit") {
    return;
  }

  if (normalized === "domcontentloaded") {
    await page.waitForFunction(
      () => document.readyState === "interactive" || document.readyState === "complete",
      null,
      { timeout, polling: 25 },
    );
    return;
  }

  if (normalized === "networkidle") {
    await page.waitForFunction(() => document.readyState === "complete", null, {
      timeout,
      polling: 25,
    });
    await sleep(100);
    return;
  }

  await page.waitForFunction(() => document.readyState === "complete", null, {
    timeout,
    polling: 25,
  });
}

export class RomLocator {
  #page;
  #selector;

  constructor(page, selector) {
    this.#page = page;
    this.#selector = String(selector);
  }

  click(options = {}) {
    return this.#page.click(this.#selector, options);
  }

  fill(value, options = {}) {
    return this.#page.fill(this.#selector, value, options);
  }

  textContent(options = {}) {
    return this.#page.textContent(this.#selector, options);
  }

  innerHTML(options = {}) {
    return this.#page.innerHTML(this.#selector, options);
  }

  waitFor(options = {}) {
    return this.#page.waitForSelector(this.#selector, options);
  }

  async evaluate(pageFunction, arg) {
    return this.#page.evaluate(
      ({ selector, arg: innerArg, pageFunctionSource }) => {
        const element = document.querySelector(selector);
        if (!element) {
          return null;
        }

        const fn = (0, eval)(`(${pageFunctionSource})`);
        return fn(element, innerArg);
      },
      {
        selector: this.#selector,
        arg,
        pageFunctionSource:
          typeof pageFunction === "function" ? pageFunction.toString() : String(pageFunction),
      },
    );
  }
}

export class RomPage {
  #runtime;

  constructor(runtime) {
    this.#runtime = runtime;
  }

  locator(selector) {
    return new RomLocator(this, selector);
  }

  async evaluate(pageFunction, arg = null) {
    assertLiveSession(this.#runtime);
    return this.#runtime.evalJson(buildEvaluateJsonScript(pageFunction, arg));
  }

  async content() {
    assertLiveSession(this.#runtime);
    return this.#runtime.evalAsync("(async () => document.documentElement.outerHTML)()");
  }

  async setContent(html, options = {}) {
    assertLiveSession(this.#runtime);
    await this.#runtime.evalAsync(buildSetContentScript(html));
    await waitForLoadState(this, options.waitUntil ?? "load", options.timeout ?? 30_000);
  }

  async goto(url, options = {}) {
    assertLiveSession(this.#runtime);
    const response = await this.#runtime.evalJson(buildGotoScript(url, options));
    this.#runtime.config = {
      ...this.#runtime.config,
      href: response.url,
    };
    await waitForLoadState(this, options.waitUntil ?? "load", options.timeout ?? 30_000);
    return response;
  }

  async waitForSelector(selector, options = {}) {
    assertLiveSession(this.#runtime);
    const timeout = Number(options.timeout ?? 30_000);
    const polling = Number(options.polling ?? 50);
    const state = String(options.state ?? "visible");
    const deadline = Date.now() + timeout;
    const normalizedSelector = String(selector);

    while (Date.now() <= deadline) {
      const status = await this.evaluate(({ selector: currentSelector, state: currentState }) => {
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
      }, { selector: normalizedSelector, state });

      if (status?.satisfied) {
        return state === "detached" || state === "hidden"
          ? null
          : new RomLocator(this, normalizedSelector);
      }

      await sleep(polling);
    }

    throw new Error(`Timed out waiting for selector: ${normalizedSelector}`);
  }

  async waitForFunction(pageFunction, arg = null, options = {}) {
    assertLiveSession(this.#runtime);
    const timeout = Number(options.timeout ?? 30_000);
    const polling = options.polling === "raf" ? 16 : Number(options.polling ?? 50);
    const deadline = Date.now() + timeout;

    while (Date.now() <= deadline) {
      const result = await this.evaluate(pageFunction, arg);
      if (result) {
        return result;
      }
      await sleep(polling);
    }

    throw new Error("Timed out waiting for function.");
  }

  async click(selector, options = {}) {
    assertLiveSession(this.#runtime);
    await this.waitForSelector(selector, {
      timeout: options.timeout,
      state: options.state ?? "attached",
      polling: options.polling,
    });

    const result = await this.evaluate((currentSelector) => {
      const element = document.querySelector(currentSelector);
      if (!element) {
        return { ok: false };
      }

      if (typeof element.focus === "function") {
        element.focus();
      }

      for (const type of ["mousedown", "mouseup", "click"]) {
        element.dispatchEvent(new Event(type, { bubbles: true, cancelable: true }));
      }

      return { ok: true };
    }, String(selector));

    if (!result?.ok) {
      throw new Error(`Failed to click selector: ${selector}`);
    }
  }

  async fill(selector, value, options = {}) {
    assertLiveSession(this.#runtime);
    await this.waitForSelector(selector, {
      timeout: options.timeout,
      state: options.state ?? "attached",
      polling: options.polling,
    });

    const result = await this.evaluate(({ selector: currentSelector, value: currentValue }) => {
      const element = document.querySelector(currentSelector);
      if (!element) {
        return { ok: false, reason: "missing" };
      }

      if (typeof element.focus === "function") {
        element.focus();
      }

      const tagName = String(element.tagName ?? "").toUpperCase();
      if (
        "value" in element ||
        tagName === "INPUT" ||
        tagName === "TEXTAREA" ||
        tagName === "SELECT"
      ) {
        element.value = String(currentValue);
      } else {
        element.textContent = String(currentValue);
      }

      element.dispatchEvent(new Event("input", { bubbles: true, cancelable: true }));
      element.dispatchEvent(new Event("change", { bubbles: true }));
      return { ok: true };
    }, { selector: String(selector), value: String(value) });

    if (!result?.ok) {
      throw new Error(`Failed to fill selector: ${selector}`);
    }
  }

  async textContent(selector, options = {}) {
    assertLiveSession(this.#runtime);
    await this.waitForSelector(selector, {
      timeout: options.timeout,
      state: options.state ?? "attached",
      polling: options.polling,
    });

    return this.evaluate((currentSelector) => {
      const element = document.querySelector(currentSelector);
      return element ? element.textContent : null;
    }, String(selector));
  }

  async innerHTML(selector, options = {}) {
    assertLiveSession(this.#runtime);
    await this.waitForSelector(selector, {
      timeout: options.timeout,
      state: options.state ?? "attached",
      polling: options.polling,
    });

    return this.evaluate((currentSelector) => {
      const element = document.querySelector(currentSelector);
      return element ? element.innerHTML : null;
    }, String(selector));
  }
}
