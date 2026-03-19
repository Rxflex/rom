    function notifyDomMutation(record) {
        if (typeof g.__rom_queueMutationRecord === "function") {
            g.__rom_queueMutationRecord(record);
        }
        if (typeof g.__rom_queueLayoutObservation === "function") {
            g.__rom_queueLayoutObservation(record.target);
        }
    }

    function notifyIframeLoad(node) {
        if (
            node instanceof HTMLIFrameElement &&
            typeof node.onload === "function"
        ) {
            Promise.resolve().then(() => node.onload());
        }
    }

    function escapeHtmlText(value) {
        return String(value)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
    }

    function escapeHtmlAttribute(value) {
        return escapeHtmlText(value).replace(/"/g, "&quot;");
    }

    function serializeNodeToHtml(node) {
        if (!node) {
            return "";
        }

        if (node.nodeType === 3) {
            return escapeHtmlText(node.textContent ?? "");
        }

        if (node.nodeType === 11 || node.nodeType === 9) {
            return node.childNodes.map((child) => serializeNodeToHtml(child)).join("");
        }

        if (node.nodeType !== 1) {
            return "";
        }

        const tagName = node.tagName.toLowerCase();
        const attributes = Array.from(node.attributes.entries())
            .map(([name, value]) => ` ${name}="${escapeHtmlAttribute(value)}"`)
            .join("");
        const content = node.childNodes.map((child) => serializeNodeToHtml(child)).join("");
        return `<${tagName}${attributes}>${content}</${tagName}>`;
    }

    function parseHtmlFragment(source) {
        const fragmentDocument = new Document();
        const parsed = parseMarkup(fragmentDocument, String(source ?? ""), false);
        if (parsed.error) {
            return [fragmentDocument.createTextNode(String(source ?? "").replace(/<[^>]*>/g, ""))];
        }
        return parsed.nodes;
    }

    function replaceNodeWithNodes(node, replacementNodes) {
        const parent = node?.parentNode ?? null;
        if (!parent) {
            return;
        }

        const index = parent.childNodes.indexOf(node);
        if (index < 0) {
            return;
        }

        const previousSibling = parent.childNodes[index - 1] ?? null;
        const nextSibling = parent.childNodes[index + 1] ?? null;

        for (const replacementNode of replacementNodes) {
            if (replacementNode.parentNode) {
                replacementNode.parentNode.removeChild(replacementNode);
            }
        }

        parent.childNodes.splice(index, 1, ...replacementNodes);
        node.parentNode = null;

        for (const replacementNode of replacementNodes) {
            replacementNode.parentNode = parent;
            notifyIframeLoad(replacementNode);
        }

        notifyDomMutation({
            type: "childList",
            target: parent,
            addedNodes: replacementNodes,
            removedNodes: [node],
            previousSibling,
            nextSibling,
        });
    }

    class Node extends EventTarget {
        constructor(nodeType, nodeName) {
            super();
            this.nodeType = nodeType;
            this.nodeName = nodeName;
            this.parentNode = null;
            this.childNodes = [];
        }

        appendChild(node) {
            if (node instanceof DocumentFragment) {
                const addedNodes = node.childNodes.slice();
                if (!addedNodes.length) {
                    return node;
                }

                const previousSibling = this.lastChild;
                node.childNodes = [];
                for (const child of addedNodes) {
                    child.parentNode = this;
                    this.childNodes.push(child);
                    notifyIframeLoad(child);
                }

                notifyDomMutation({
                    type: "childList",
                    target: this,
                    addedNodes,
                    removedNodes: [],
                    previousSibling,
                    nextSibling: null,
                });

                return node;
            }

            if (node.parentNode) {
                node.parentNode.removeChild(node);
            }
            const previousSibling = this.lastChild;
            node.parentNode = this;
            this.childNodes.push(node);
            notifyDomMutation({
                type: "childList",
                target: this,
                addedNodes: [node],
                removedNodes: [],
                previousSibling,
                nextSibling: null,
            });
            notifyIframeLoad(node);

            return node;
        }

        removeChild(node) {
            const index = this.childNodes.indexOf(node);
            if (index >= 0) {
                const previousSibling = this.childNodes[index - 1] ?? null;
                const nextSibling = this.childNodes[index + 1] ?? null;
                this.childNodes.splice(index, 1);
                node.parentNode = null;
                notifyDomMutation({
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
            notifyDomMutation({
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
            notifyDomMutation({
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
            notifyDomMutation({
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
            return this.childNodes.map((child) => serializeNodeToHtml(child)).join("");
        }

        set innerHTML(value) {
            while (this.childNodes.length > 0) {
                this.removeChild(this.childNodes[this.childNodes.length - 1]);
            }

            for (const node of parseHtmlFragment(value)) {
                this.appendChild(node);
            }
        }

        get outerHTML() {
            return serializeNodeToHtml(this);
        }

        set outerHTML(value) {
            replaceNodeWithNodes(this, parseHtmlFragment(value));
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

    class DocumentFragment extends Node {
        constructor() {
            super(11, "#document-fragment");
        }

        cloneNode(deep = false) {
            const clone = new DocumentFragment();
            if (deep) {
                for (const child of this.childNodes) {
                    clone.appendChild(child.cloneNode(true));
                }
            }
            return clone;
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

        get children() {
            return this.childNodes.filter((node) => node.nodeType === 1);
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
            return new DocumentFragment();
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

    g.DocumentFragment = DocumentFragment;

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

    const document = new Document();
    document.defaultView = g;
