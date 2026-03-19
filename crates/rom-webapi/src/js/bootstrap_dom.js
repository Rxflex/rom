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

        if (node.nodeType === 8) {
            return `<!--${String(node.data ?? "").replace(/-->/g, "--&gt;")}-->`;
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

    function normalizeInsertionNodes(nodes) {
        const normalized = [];
        for (const node of nodes) {
            if (node instanceof DocumentFragment) {
                const fragmentChildren = node.childNodes.slice();
                node.childNodes = [];
                normalized.push(...fragmentChildren);
                continue;
            }

            normalized.push(typeof node === "string" ? new Text(node) : node);
        }
        return normalized;
    }

    function mutateChildList(parent, index, insertedNodes, removedNodes) {
        const normalizedInsertedNodes = normalizeInsertionNodes(insertedNodes);
        if (index < 0) {
            return normalizedInsertedNodes;
        }

        const previousSibling = parent.childNodes[index - 1] ?? null;
        const nextSibling = parent.childNodes[index + removedNodes.length] ?? null;

        for (const insertedNode of normalizedInsertedNodes) {
            if (insertedNode.parentNode) {
                insertedNode.parentNode.removeChild(insertedNode);
            }
        }

        parent.childNodes.splice(index, removedNodes.length, ...normalizedInsertedNodes);
        for (const removedNode of removedNodes) {
            removedNode.parentNode = null;
        }

        for (const insertedNode of normalizedInsertedNodes) {
            insertedNode.parentNode = parent;
            assignOwnerDocumentToSubtree(
                insertedNode,
                parent.nodeType === 9 ? parent : parent.ownerDocument,
            );
            notifyIframeLoad(insertedNode);
        }

        if (normalizedInsertedNodes.length > 0 || removedNodes.length > 0) {
            notifyDomMutation({
                type: "childList",
                target: parent,
                addedNodes: normalizedInsertedNodes,
                removedNodes,
                previousSibling,
                nextSibling,
            });
        }

        return normalizedInsertedNodes;
    }

    function replaceNodeWithNodes(node, replacementNodes) {
        const parent = node?.parentNode ?? null;
        if (!parent) {
            return;
        }

        const index = parent.childNodes.indexOf(node);
        mutateChildList(parent, index, replacementNodes, [node]);
    }

    function insertAdjacentNodes(target, position, nodes) {
        const normalizedPosition = String(position).toLowerCase();

        if (normalizedPosition === "beforebegin") {
            if (!target.parentNode) {
                return [];
            }
            const index = target.parentNode.childNodes.indexOf(target);
            return mutateChildList(target.parentNode, index, nodes, []);
        }

        if (normalizedPosition === "afterend") {
            if (!target.parentNode) {
                return [];
            }
            const index = target.parentNode.childNodes.indexOf(target);
            return mutateChildList(target.parentNode, index + 1, nodes, []);
        }

        if (normalizedPosition === "afterbegin") {
            return mutateChildList(target, 0, nodes, []);
        }

        if (normalizedPosition === "beforeend") {
            return mutateChildList(target, target.childNodes.length, nodes, []);
        }

        throw new SyntaxError(
            "Failed to execute 'insertAdjacentHTML': invalid position.",
        );
    }

    function collectElementDescendants(root, predicate) {
        const matches = [];

        walk(root, (node) => {
            if (node?.nodeType === 1 && predicate(node)) {
                matches.push(node);
            }
            return false;
        });

        return matches;
    }

    function getElementsByTagNameFrom(root, tagName) {
        const normalized = String(tagName).toLowerCase();
        return collectElementDescendants(root, (node) => (
            normalized === "*" || node.tagName.toLowerCase() === normalized
        ));
    }

    function getElementsByClassNameFrom(root, classNames) {
        const requiredTokens = String(classNames)
            .trim()
            .split(/\s+/)
            .filter(Boolean);

        if (!requiredTokens.length) {
            return [];
        }

        return collectElementDescendants(root, (node) => {
            const classTokens = (node.className ?? "")
                .split(/\s+/)
                .filter(Boolean);
            return requiredTokens.every((token) => classTokens.includes(token));
        });
    }

    function getOwnerDocumentForNode(node) {
        if (!node) {
            return null;
        }
        if (node.nodeType === 9) {
            return null;
        }
        if (node.__ownerDocument) {
            return node.__ownerDocument;
        }
        if (node.parentNode) {
            return node.parentNode.nodeType === 9
                ? node.parentNode
                : node.parentNode.ownerDocument;
        }
        return null;
    }

    function assignOwnerDocumentToSubtree(node, ownerDocument) {
        if (!node || node.nodeType === 9) {
            return;
        }

        node.__ownerDocument = ownerDocument;
        for (const child of node.childNodes ?? []) {
            assignOwnerDocumentToSubtree(child, ownerDocument);
        }
    }

    function getBaseUriForNode(node) {
        const documentNode = node?.nodeType === 9 ? node : node?.ownerDocument ?? null;
        const defaultView = documentNode?.defaultView ?? null;
        const href = defaultView?.location?.href;
        return typeof href === "string" ? href : "about:blank";
    }

    function ownStyleKeys(style) {
        return Object.keys(style ?? {}).filter((key) => typeof style[key] !== "function");
    }

    function areNodeAttributesEqual(leftNode, rightNode) {
        const leftAttributes = leftNode?.attributes;
        const rightAttributes = rightNode?.attributes;

        if (!(leftAttributes instanceof Map) || !(rightAttributes instanceof Map)) {
            return true;
        }

        if (leftAttributes.size !== rightAttributes.size) {
            return false;
        }

        for (const [name, value] of leftAttributes.entries()) {
            if (rightAttributes.get(name) !== value) {
                return false;
            }
        }

        return true;
    }

    function areNodeStylesEqual(leftNode, rightNode) {
        const leftKeys = ownStyleKeys(leftNode?.style).sort();
        const rightKeys = ownStyleKeys(rightNode?.style).sort();

        if (leftKeys.length !== rightKeys.length) {
            return false;
        }

        for (let index = 0; index < leftKeys.length; index += 1) {
            if (leftKeys[index] !== rightKeys[index]) {
                return false;
            }

            if (leftNode.style[leftKeys[index]] !== rightNode.style[rightKeys[index]]) {
                return false;
            }
        }

        return true;
    }

    function areNodesEqual(leftNode, rightNode) {
        if (leftNode === rightNode) {
            return true;
        }

        if (!leftNode || !rightNode) {
            return false;
        }

        if (
            leftNode.nodeType !== rightNode.nodeType ||
            leftNode.nodeName !== rightNode.nodeName ||
            leftNode.nodeValue !== rightNode.nodeValue
        ) {
            return false;
        }

        if (!areNodeAttributesEqual(leftNode, rightNode)) {
            return false;
        }

        if (!areNodeStylesEqual(leftNode, rightNode)) {
            return false;
        }

        if ((leftNode.childNodes?.length ?? 0) !== (rightNode.childNodes?.length ?? 0)) {
            return false;
        }

        for (let index = 0; index < (leftNode.childNodes?.length ?? 0); index += 1) {
            if (!areNodesEqual(leftNode.childNodes[index], rightNode.childNodes[index])) {
                return false;
            }
        }

        return true;
    }

    class Node extends EventTarget {
        constructor(nodeType, nodeName) {
            super();
            this.nodeType = nodeType;
            this.nodeName = nodeName;
            this.parentNode = null;
            this.childNodes = [];
            this.__ownerDocument = null;
        }

        appendChild(node) {
            mutateChildList(this, this.childNodes.length, [node], []);
            return node;
        }

        insertBefore(node, referenceNode = null) {
            if (referenceNode === null) {
                return this.appendChild(node);
            }

            const referenceIndex = this.childNodes.indexOf(referenceNode);
            if (referenceIndex < 0) {
                return node;
            }

            mutateChildList(this, referenceIndex, [node], []);
            return node;
        }

        removeChild(node) {
            const index = this.childNodes.indexOf(node);
            if (index >= 0) {
                mutateChildList(this, index, [], [node]);
            }
            return node;
        }

        replaceChild(newChild, oldChild) {
            const index = this.childNodes.indexOf(oldChild);
            if (index >= 0) {
                mutateChildList(this, index, [newChild], [oldChild]);
            }
            return oldChild;
        }

        remove() {
            if (this.parentNode) {
                this.parentNode.removeChild(this);
            }
        }

        prepend(...nodes) {
            mutateChildList(this, 0, nodes, []);
        }

        replaceChildren(...nodes) {
            const removedNodes = this.childNodes.slice();
            mutateChildList(this, 0, nodes, removedNodes);
        }

        before(...nodes) {
            if (!this.parentNode) {
                return;
            }
            const index = this.parentNode.childNodes.indexOf(this);
            mutateChildList(this.parentNode, index, nodes, []);
        }

        after(...nodes) {
            if (!this.parentNode) {
                return;
            }
            const index = this.parentNode.childNodes.indexOf(this);
            mutateChildList(this.parentNode, index + 1, nodes, []);
        }

        replaceWith(...nodes) {
            replaceNodeWithNodes(this, nodes);
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

        get parentElement() {
            return this.parentNode?.nodeType === 1 ? this.parentNode : null;
        }

        get previousSibling() {
            if (!this.parentNode) {
                return null;
            }
            const index = this.parentNode.childNodes.indexOf(this);
            return index > 0 ? this.parentNode.childNodes[index - 1] : null;
        }

        get nextSibling() {
            if (!this.parentNode) {
                return null;
            }
            const index = this.parentNode.childNodes.indexOf(this);
            return index >= 0 ? this.parentNode.childNodes[index + 1] ?? null : null;
        }

        get ownerDocument() {
            return getOwnerDocumentForNode(this);
        }

        get isConnected() {
            let current = this;
            while (current) {
                if (current.nodeType === 9) {
                    return true;
                }
                current = current.parentNode ?? null;
            }
            return false;
        }

        get baseURI() {
            return getBaseUriForNode(this);
        }

        get nodeValue() {
            return null;
        }

        set nodeValue(_value) {}

        get textContent() {
            return this.childNodes
                .map((child) => (child?.nodeType === 8 ? "" : child.textContent ?? ""))
                .join("");
        }

        set textContent(value) {
            while (this.childNodes.length > 0) {
                this.removeChild(this.childNodes[this.childNodes.length - 1]);
            }
            if (value !== null && value !== undefined && value !== "") {
                this.appendChild(new Text(value));
            }
        }

        contains(node) {
            let current = node ?? null;
            while (current) {
                if (current === this) {
                    return true;
                }
                current = current.parentNode ?? null;
            }
            return false;
        }

        hasChildNodes() {
            return this.childNodes.length > 0;
        }

        getRootNode(_options = undefined) {
            let current = this;
            while (current?.parentNode) {
                current = current.parentNode;
            }
            return current ?? this;
        }

        isSameNode(otherNode) {
            return this === otherNode;
        }

        isEqualNode(otherNode) {
            return areNodesEqual(this, otherNode);
        }

        normalize() {
            let index = 0;
            while (index < this.childNodes.length) {
                const child = this.childNodes[index];

                if (child?.nodeType === 3) {
                    if (child.data === "") {
                        this.removeChild(child);
                        continue;
                    }

                    while (
                        this.childNodes[index + 1]?.nodeType === 3
                    ) {
                        const nextText = this.childNodes[index + 1];
                        child.textContent = child.data + nextText.data;
                        this.removeChild(nextText);
                    }

                    index += 1;
                    continue;
                }

                if (typeof child?.normalize === "function") {
                    child.normalize();
                }
                index += 1;
            }
        }
    }

    class Text extends Node {
        constructor(data = "") {
            super(3, "#text");
            this.data = String(data);
        }

        cloneNode() {
            const clone = new Text(this.data);
            clone.__ownerDocument = this.ownerDocument;
            return clone;
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

        get nodeValue() {
            return this.data;
        }

        set nodeValue(value) {
            this.textContent = value;
        }

        get wholeText() {
            const segments = [this.data];

            let previous = this.previousSibling;
            while (previous?.nodeType === 3) {
                segments.unshift(previous.data);
                previous = previous.previousSibling;
            }

            let next = this.nextSibling;
            while (next?.nodeType === 3) {
                segments.push(next.data);
                next = next.nextSibling;
            }

            return segments.join("");
        }

        splitText(offset) {
            const numericOffset = Number(offset);
            const normalizedOffset = Number.isFinite(numericOffset)
                ? Math.trunc(numericOffset)
                : 0;

            if (normalizedOffset < 0 || normalizedOffset > this.data.length) {
                throw new DOMException(
                    "The offset is outside the text node bounds.",
                    "IndexSizeError",
                );
            }

            const prefix = this.data.slice(0, normalizedOffset);
            const suffix = this.data.slice(normalizedOffset);
            const splitNode = new Text(suffix);
            splitNode.__ownerDocument = this.ownerDocument;

            this.textContent = prefix;

            if (this.parentNode) {
                this.parentNode.insertBefore(splitNode, this.nextSibling);
            }

            return splitNode;
        }
    }

    class Comment extends Node {
        constructor(data = "") {
            super(8, "#comment");
            this.data = String(data);
        }

        cloneNode() {
            const clone = new Comment(this.data);
            clone.__ownerDocument = this.ownerDocument;
            return clone;
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

        get nodeValue() {
            return this.data;
        }

        set nodeValue(value) {
            this.textContent = value;
        }
    }

    class DOMTokenList {
        constructor(element, attributeName) {
            this.__element = element;
            this.__attributeName = String(attributeName);
        }

        __tokens() {
            return (this.__element.getAttribute(this.__attributeName) ?? "")
                .split(/\s+/)
                .filter(Boolean);
        }

        __setTokens(tokens) {
            if (!tokens.length) {
                this.__element.removeAttribute(this.__attributeName);
                return;
            }
            this.__element.setAttribute(this.__attributeName, tokens.join(" "));
        }

        __normalizeToken(token) {
            const normalized = String(token ?? "").trim();
            if (!normalized || /\s/.test(normalized)) {
                throw new SyntaxError("The token provided must not be empty or contain whitespace.");
            }
            return normalized;
        }

        get length() {
            return this.__tokens().length;
        }

        get value() {
            return this.__tokens().join(" ");
        }

        set value(nextValue) {
            this.__setTokens(
                String(nextValue ?? "")
                    .split(/\s+/)
                    .filter(Boolean),
            );
        }

        item(index) {
            return this.__tokens()[Number(index)] ?? null;
        }

        contains(token) {
            return this.__tokens().includes(this.__normalizeToken(token));
        }

        add(...tokens) {
            const nextTokens = this.__tokens();
            for (const token of tokens.map((value) => this.__normalizeToken(value))) {
                if (!nextTokens.includes(token)) {
                    nextTokens.push(token);
                }
            }
            this.__setTokens(nextTokens);
        }

        remove(...tokens) {
            const removed = new Set(tokens.map((value) => this.__normalizeToken(value)));
            this.__setTokens(this.__tokens().filter((token) => !removed.has(token)));
        }

        toggle(token, force = undefined) {
            const normalized = this.__normalizeToken(token);
            const tokens = this.__tokens();
            const present = tokens.includes(normalized);
            const shouldAdd = force === undefined ? !present : Boolean(force);

            if (shouldAdd && !present) {
                tokens.push(normalized);
                this.__setTokens(tokens);
            } else if (!shouldAdd && present) {
                this.__setTokens(tokens.filter((entry) => entry !== normalized));
            }

            return shouldAdd;
        }

        replace(oldToken, newToken) {
            const normalizedOld = this.__normalizeToken(oldToken);
            const normalizedNew = this.__normalizeToken(newToken);
            const tokens = this.__tokens();
            const index = tokens.indexOf(normalizedOld);
            if (index < 0) {
                return false;
            }

            if (!tokens.includes(normalizedNew)) {
                tokens[index] = normalizedNew;
            } else {
                tokens.splice(index, 1);
            }
            this.__setTokens(tokens);
            return true;
        }

        toString() {
            return this.value;
        }
    }

    function datasetAttributeToPropertyName(attributeName) {
        const normalized = String(attributeName);
        if (!normalized.startsWith("data-") || normalized.length <= 5) {
            return null;
        }

        return normalized.slice(5).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    }

    function datasetPropertyToAttributeName(propertyName) {
        return `data-${String(propertyName).replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`;
    }

    class DOMStringMap {
        constructor(element) {
            this.__element = element;
        }

        __entries() {
            const entries = [];
            for (const [name, value] of this.__element.attributes.entries()) {
                const propertyName = datasetAttributeToPropertyName(name);
                if (propertyName !== null) {
                    entries.push([propertyName, value]);
                }
            }
            return entries;
        }

        __keys() {
            return this.__entries().map(([propertyName]) => propertyName);
        }

        __has(propertyName) {
            return this.__element.hasAttribute(datasetPropertyToAttributeName(propertyName));
        }

        __get(propertyName) {
            return this.__element.getAttribute(datasetPropertyToAttributeName(propertyName));
        }

        __set(propertyName, value) {
            this.__element.setAttribute(datasetPropertyToAttributeName(propertyName), String(value));
        }

        __delete(propertyName) {
            this.__element.removeAttribute(datasetPropertyToAttributeName(propertyName));
        }

        toString() {
            return "[object DOMStringMap]";
        }
    }

    function createDatasetProxy(element) {
        const target = new DOMStringMap(element);
        return new Proxy(target, {
            get(currentTarget, property, receiver) {
                if (typeof property === "string" && !(property in currentTarget)) {
                    return currentTarget.__get(property);
                }
                return Reflect.get(currentTarget, property, receiver);
            },
            set(currentTarget, property, value, receiver) {
                if (typeof property === "string" && !(property in currentTarget)) {
                    currentTarget.__set(property, value);
                    return true;
                }
                return Reflect.set(currentTarget, property, value, receiver);
            },
            deleteProperty(currentTarget, property) {
                if (typeof property === "string" && !(property in currentTarget)) {
                    currentTarget.__delete(property);
                    return true;
                }
                return Reflect.deleteProperty(currentTarget, property);
            },
            has(currentTarget, property) {
                if (typeof property === "string" && !(property in currentTarget)) {
                    return currentTarget.__has(property);
                }
                return Reflect.has(currentTarget, property);
            },
            ownKeys(currentTarget) {
                return currentTarget.__keys();
            },
            getOwnPropertyDescriptor(currentTarget, property) {
                if (typeof property === "string" && currentTarget.__has(property)) {
                    return {
                        configurable: true,
                        enumerable: true,
                        value: currentTarget.__get(property),
                        writable: true,
                    };
                }
                return Reflect.getOwnPropertyDescriptor(currentTarget, property);
            },
        });
    }

    class Element extends Node {
        constructor(tagName = "div") {
            const normalized = String(tagName).toUpperCase();
            super(1, normalized);
            this.tagName = normalized;
            this.attributes = new Map();
            this.style = createStyleDeclaration();
            this.__classList = new DOMTokenList(this, "class");
            this.__dataset = createDatasetProxy(this);
        }

        cloneNode(deep = false) {
            const clone = new this.constructor(this.tagName);
            clone.__ownerDocument = this.ownerDocument;
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

        get classList() {
            return this.__classList;
        }

        get dataset() {
            return this.__dataset;
        }

        get children() {
            return this.childNodes.filter((node) => node.nodeType === 1);
        }

        get firstElementChild() {
            return this.children[0] ?? null;
        }

        get lastElementChild() {
            return this.children[this.children.length - 1] ?? null;
        }

        get childElementCount() {
            return this.children.length;
        }

        get previousElementSibling() {
            let current = this.previousSibling;
            while (current && current.nodeType !== 1) {
                current = current.previousSibling;
            }
            return current ?? null;
        }

        get nextElementSibling() {
            let current = this.nextSibling;
            while (current && current.nodeType !== 1) {
                current = current.nextSibling;
            }
            return current ?? null;
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

        insertAdjacentHTML(position, html) {
            insertAdjacentNodes(this, position, parseHtmlFragment(html));
        }

        insertAdjacentText(position, text) {
            insertAdjacentNodes(this, position, [new Text(text)]);
        }

        insertAdjacentElement(position, element) {
            const inserted = insertAdjacentNodes(this, position, [element]);
            return inserted[0] ?? null;
        }

        matches(selector) {
            return matchesSelector(this, String(selector));
        }

        closest(selector) {
            const normalized = String(selector);
            let current = this;
            while (current) {
                if (matchesSelector(current, normalized)) {
                    return current;
                }
                current = current.parentNode ?? null;
            }
            return null;
        }

        querySelector(selector) {
            return querySelectorFrom(this, String(selector));
        }

        querySelectorAll(selector) {
            return querySelectorAllFrom(this, String(selector));
        }

        getElementsByTagName(tagName) {
            return getElementsByTagNameFrom(this, tagName);
        }

        getElementsByClassName(classNames) {
            return getElementsByClassNameFrom(this, classNames);
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
            clone.__ownerDocument = this.ownerDocument;
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

        getElementsByTagName(tagName) {
            return getElementsByTagNameFrom(this, tagName);
        }

        getElementsByClassName(classNames) {
            return getElementsByClassNameFrom(this, classNames);
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
            assignOwnerDocumentToSubtree(this.documentElement, this);
            assignOwnerDocumentToSubtree(this.head, this);
            assignOwnerDocumentToSubtree(this.body, this);
            this.appendChild(this.documentElement);
            this.documentElement.appendChild(this.head);
            this.documentElement.appendChild(this.body);
        }

        createElement(tagName) {
            const normalized = String(tagName).toLowerCase();
            let element = null;
            if (normalized === "canvas") {
                element = new HTMLCanvasElement();
            } else if (normalized === "iframe") {
                element = new HTMLIFrameElement();
            } else {
                element = new Element(tagName);
            }
            assignOwnerDocumentToSubtree(element, this);
            return element;
        }

        createTextNode(data) {
            const node = new Text(data);
            assignOwnerDocumentToSubtree(node, this);
            return node;
        }

        createComment(data) {
            const node = new Comment(data);
            assignOwnerDocumentToSubtree(node, this);
            return node;
        }

        createEvent(type) {
            return new Event(type);
        }

        createDocumentFragment() {
            const fragment = new DocumentFragment();
            assignOwnerDocumentToSubtree(fragment, this);
            return fragment;
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

        getElementsByTagName(tagName) {
            return getElementsByTagNameFrom(this, tagName);
        }

        getElementsByClassName(classNames) {
            return getElementsByClassNameFrom(this, classNames);
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
