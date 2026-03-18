    const mutationObservers = new Set();

    class Node extends EventTarget {
        constructor(nodeType, nodeName) {
            super();
            this.nodeType = nodeType;
            this.nodeName = nodeName;
            this.parentNode = null;
            this.childNodes = [];
        }

        appendChild(node) {
            if (node.parentNode) {
                node.parentNode.removeChild(node);
            }
            const previousSibling = this.lastChild;
            node.parentNode = this;
            this.childNodes.push(node);
            queueMutationRecord({
                type: "childList",
                target: this,
                addedNodes: [node],
                removedNodes: [],
                previousSibling,
                nextSibling: null,
            });

            if (
                node instanceof HTMLIFrameElement &&
                typeof node.onload === "function"
            ) {
                Promise.resolve().then(() => node.onload());
            }

            return node;
        }

        removeChild(node) {
            const index = this.childNodes.indexOf(node);
            if (index >= 0) {
                const previousSibling = this.childNodes[index - 1] ?? null;
                const nextSibling = this.childNodes[index + 1] ?? null;
                this.childNodes.splice(index, 1);
                node.parentNode = null;
                queueMutationRecord({
                    type: "childList",
                    target: this,
                    addedNodes: [],
                    removedNodes: [node],
                    previousSibling,
                    nextSibling,
                });
            }
            return node;
        }

        cloneNode(deep = false) {
            const clone = Object.assign(
                Object.create(Object.getPrototypeOf(this)),
                typeof structuredClone === "function" ? structuredClone(this) : { ...this },
            );
            clone.parentNode = null;
            clone.childNodes = [];

            if (deep) {
                for (const child of this.childNodes) {
                    clone.appendChild(child.cloneNode(true));
                }
            }

            return clone;
        }

        get firstChild() {
            return this.childNodes[0] ?? null;
        }

        get lastChild() {
            return this.childNodes[this.childNodes.length - 1] ?? null;
        }

        get textContent() {
            return this.childNodes.map((child) => child.textContent ?? "").join("");
        }

        set textContent(value) {
            while (this.childNodes.length > 0) {
                this.removeChild(this.childNodes[this.childNodes.length - 1]);
            }
            if (value !== null && value !== undefined && value !== "") {
                this.appendChild(new Text(value));
            }
        }
    }

    class Text extends Node {
        constructor(data = "") {
            super(3, "#text");
            this.data = String(data);
        }

        cloneNode() {
            return new Text(this.data);
        }

        get textContent() {
            return this.data;
        }

        set textContent(value) {
            const oldValue = this.data;
            this.data = String(value);
            queueMutationRecord({
                type: "characterData",
                target: this,
                oldValue,
            });
        }
    }

    class Element extends Node {
        constructor(tagName = "div") {
            const normalized = String(tagName).toUpperCase();
            super(1, normalized);
            this.tagName = normalized;
            this.attributes = new Map();
            this.style = createStyleDeclaration();
        }

        cloneNode(deep = false) {
            const clone = new this.constructor(this.tagName);
            for (const [key, value] of this.attributes.entries()) {
                clone.setAttribute(key, value);
            }
            clone.style = Object.assign(createStyleDeclaration(), this.style);
            if (deep) {
                for (const child of this.childNodes) {
                    clone.appendChild(child.cloneNode(true));
                }
            }
            return clone;
        }

        setAttribute(name, value) {
            const normalizedName = String(name);
            const oldValue = this.getAttribute(normalizedName);
            this.attributes.set(normalizedName, String(value));
            queueMutationRecord({
                type: "attributes",
                target: this,
                attributeName: normalizedName,
                oldValue,
            });
        }

        getAttribute(name) {
            return this.attributes.get(String(name)) ?? null;
        }

        hasAttribute(name) {
            return this.attributes.has(String(name));
        }

        removeAttribute(name) {
            const normalizedName = String(name);
            const oldValue = this.getAttribute(normalizedName);
            this.attributes.delete(normalizedName);
            queueMutationRecord({
                type: "attributes",
                target: this,
                attributeName: normalizedName,
                oldValue,
            });
        }

        get id() {
            return this.getAttribute("id") ?? "";
        }

        set id(value) {
            this.setAttribute("id", value);
        }

        get className() {
            return this.getAttribute("class") ?? "";
        }

        set className(value) {
            this.setAttribute("class", value);
        }

        get children() {
            return this.childNodes.filter((node) => node.nodeType === 1);
        }

        get offsetWidth() {
            return Math.max(1, this.textContent.length * 8 + this.children.length * 4);
        }

        get offsetHeight() {
            return Math.max(1, 16 + this.children.length * 8);
        }

        get offsetParent() {
            return this.parentNode;
        }

        get innerHTML() {
            return this.textContent;
        }

        set innerHTML(value) {
            this.textContent = value;
        }

        querySelector(selector) {
            return querySelectorFrom(this, String(selector));
        }

        querySelectorAll(selector) {
            return querySelectorAllFrom(this, String(selector));
        }

        append(...nodes) {
            for (const node of nodes) {
                this.appendChild(
                    typeof node === "string" ? new Text(node) : node,
                );
            }
        }

        getBoundingClientRect() {
            return {
                x: 0,
                y: 0,
                width: this.offsetWidth,
                height: this.offsetHeight,
                top: 0,
                left: 0,
                right: this.offsetWidth,
                bottom: this.offsetHeight,
            };
        }
    }

    class HTMLCanvasElement extends Element {
        constructor() {
            super("canvas");
            this.width = 300;
            this.height = 150;
        }

        getContext(kind) {
            const context = createCanvasContext(kind);
            context.canvas = this;
            return context;
        }

        toDataURL() {
            return "data:image/png;base64,Uk9N";
        }
    }

    function createIframeWindow() {
        const frameWindow = Object.create(g);
        const frameDocument = new Document();
        frameWindow.window = frameWindow;
        frameWindow.self = frameWindow;
        frameWindow.parent = g;
        frameWindow.top = g.top ?? g;
        frameWindow.document = frameDocument;
        frameWindow.devicePixelRatio = 1;
        frameWindow.innerWidth = g.innerWidth;
        frameWindow.innerHeight = g.innerHeight;
        frameWindow.outerWidth = g.outerWidth;
        frameWindow.outerHeight = g.outerHeight;
        frameWindow.visualViewport = g.visualViewport;
        frameWindow.screen = screen;
        frameWindow.navigator = navigator;
        frameWindow.location = {
            href: "about:blank",
            origin: "null",
            protocol: "about:",
            host: "",
            hostname: "",
            pathname: "blank",
            search: "",
            hash: "",
        };
        frameDocument.defaultView = frameWindow;
        bindDocumentCookie(frameDocument, frameWindow.location);
        return frameWindow;
    }

    class HTMLIFrameElement extends Element {
        constructor() {
            super("iframe");
            this.src = "about:blank";
            this.srcdoc = "";
            this.onload = null;
            this.onerror = null;
            this.contentWindow = createIframeWindow();
            this.contentDocument = this.contentWindow.document;
        }
    }

    class Document extends Node {
        constructor() {
            super(9, "#document");
            this.readyState = "complete";
            this.visibilityState = "visible";
            this.hidden = false;
            this.documentElement = new Element("html");
            this.head = new Element("head");
            this.body = new Element("body");
            this.appendChild(this.documentElement);
            this.documentElement.appendChild(this.head);
            this.documentElement.appendChild(this.body);
        }

        createElement(tagName) {
            const normalized = String(tagName).toLowerCase();
            if (normalized === "canvas") {
                return new HTMLCanvasElement();
            }
            if (normalized === "iframe") {
                return new HTMLIFrameElement();
            }
            return new Element(tagName);
        }

        createTextNode(data) {
            return new Text(data);
        }

        createEvent(type) {
            return new Event(type);
        }

        createDocumentFragment() {
            return new Element("fragment");
        }

        querySelector(selector) {
            const normalized = String(selector);
            if (matchesSelector(this.documentElement, normalized)) {
                return this.documentElement;
            }
            return this.documentElement.querySelector(normalized);
        }

        querySelectorAll(selector) {
            const normalized = String(selector);
            const matches = this.documentElement.querySelectorAll(normalized);
            if (matchesSelector(this.documentElement, normalized)) {
                return [this.documentElement, ...matches];
            }
            return matches;
        }

        getElementById(id) {
            return querySelectorFrom(this, `#${id}`);
        }
    }

    class Storage {
        constructor() {
            this.__store = new Map();
        }

        get length() {
            return this.__store.size;
        }

        key(index) {
            return Array.from(this.__store.keys())[index] ?? null;
        }

        getItem(key) {
            return this.__store.has(String(key)) ? this.__store.get(String(key)) : null;
        }

        setItem(key, value) {
            this.__store.set(String(key), String(value));
        }

        removeItem(key) {
            this.__store.delete(String(key));
        }

        clear() {
            this.__store.clear();
        }
    }

    class ObserverBase {
        constructor(callback) {
            this.callback = typeof callback === "function" ? callback : () => {};
            this.targets = [];
        }

        observe(target) {
            this.targets.push(target);
        }

        unobserve(target) {
            this.targets = this.targets.filter((entry) => entry !== target);
        }

        disconnect() {
            this.targets = [];
        }

        takeRecords() {
            return [];
        }
    }

    class MutationObserver extends ObserverBase {
        constructor(callback) {
            super(callback);
            this.__records = [];
            this.__scheduled = false;
            mutationObservers.add(this);
        }

        observe(target, options = {}) {
            const normalized = normalizeMutationObserverOptions(options);
            const existing = this.targets.find((entry) => entry.target === target);
            if (existing) {
                existing.options = normalized;
                return;
            }
            this.targets.push({ target, options: normalized });
        }

        disconnect() {
            this.targets = [];
            this.__records = [];
            this.__scheduled = false;
        }

        takeRecords() {
            const records = this.__records.slice();
            this.__records = [];
            return records;
        }

        __enqueue(record) {
            this.__records.push(record);
            if (this.__scheduled) {
                return;
            }

            this.__scheduled = true;
            queueMicrotask(() => {
                this.__scheduled = false;
                if (!this.targets.length || !this.__records.length) {
                    this.__records = [];
                    return;
                }

                const records = this.takeRecords();
                this.callback(records, this);
            });
        }
    }

    function normalizeMutationObserverOptions(options) {
        return {
            childList: Boolean(options.childList),
            attributes: Boolean(options.attributes),
            characterData: Boolean(options.characterData),
            subtree: Boolean(options.subtree),
            attributeOldValue: Boolean(options.attributeOldValue),
            characterDataOldValue: Boolean(options.characterDataOldValue),
            attributeFilter: Array.isArray(options.attributeFilter)
                ? options.attributeFilter.map(String)
                : null,
        };
    }

    function queueMutationRecord(record) {
        for (const observer of mutationObservers) {
            const queuedRecord = createObserverRecord(observer, record);
            if (queuedRecord) {
                observer.__enqueue(queuedRecord);
            }
        }
    }

    function createObserverRecord(observer, record) {
        for (const entry of observer.targets) {
            if (!matchesObservedTarget(record.target, entry.target, entry.options.subtree)) {
                continue;
            }

            if (record.type === "childList" && entry.options.childList) {
                return buildMutationRecord(record, null);
            }

            if (record.type === "attributes" && entry.options.attributes) {
                if (
                    entry.options.attributeFilter &&
                    !entry.options.attributeFilter.includes(record.attributeName)
                ) {
                    return null;
                }
                return buildMutationRecord(
                    record,
                    entry.options.attributeOldValue ? record.oldValue ?? null : null,
                );
            }

            if (record.type === "characterData" && entry.options.characterData) {
                return buildMutationRecord(
                    record,
                    entry.options.characterDataOldValue ? record.oldValue ?? null : null,
                );
            }
        }

        return null;
    }

    function matchesObservedTarget(target, observedTarget, subtree) {
        if (target === observedTarget) {
            return true;
        }
        if (!subtree) {
            return false;
        }

        let current = target?.parentNode ?? null;
        while (current) {
            if (current === observedTarget) {
                return true;
            }
            current = current.parentNode;
        }
        return false;
    }

    function buildMutationRecord(record, oldValue) {
        return {
            type: record.type,
            target: record.target,
            addedNodes: record.addedNodes ?? [],
            removedNodes: record.removedNodes ?? [],
            previousSibling: record.previousSibling ?? null,
            nextSibling: record.nextSibling ?? null,
            attributeName: record.attributeName ?? null,
            oldValue,
        };
    }

    const document = new Document();
    document.defaultView = g;
