(() => {
    const g = globalThis;
    const config = g.__rom_config ?? {};
    const navigatorConfig = config.navigator ?? {};
    const locationConfig = config.location ?? {};
    const documentConfig = config.document ?? {};
    const initialLocalStorage = documentConfig.localStorage ?? null;
    const initialSessionStorage = documentConfig.sessionStorage ?? null;
    const nowBase = Date.now();

    class Event {
        constructor(type, init = {}) {
            this.type = String(type ?? "");
            this.bubbles = Boolean(init.bubbles);
            this.cancelable = Boolean(init.cancelable);
            this.composed = Boolean(init.composed);
            this.defaultPrevented = false;
            this.target = null;
            this.currentTarget = null;
            this.eventPhase = Event.NONE;
            this.__dispatchPath = [];
            this.__stopPropagation = false;
            this.__stopImmediatePropagation = false;
        }

        preventDefault() {
            if (this.cancelable) {
                this.defaultPrevented = true;
            }
        }

        stopPropagation() {
            this.__stopPropagation = true;
        }

        stopImmediatePropagation() {
            this.__stopPropagation = true;
            this.__stopImmediatePropagation = true;
        }

        composedPath() {
            return this.__dispatchPath.slice();
        }
    }

    Event.NONE = 0;
    Event.CAPTURING_PHASE = 1;
    Event.AT_TARGET = 2;
    Event.BUBBLING_PHASE = 3;
    Event.prototype.NONE = Event.NONE;
    Event.prototype.CAPTURING_PHASE = Event.CAPTURING_PHASE;
    Event.prototype.AT_TARGET = Event.AT_TARGET;
    Event.prototype.BUBBLING_PHASE = Event.BUBBLING_PHASE;

    class CustomEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.detail = init.detail ?? null;
        }
    }

    class EventTarget {
        constructor() {
            this.__listeners = new Map();
        }

        addEventListener(type, listener, options = false) {
            if (!isEventListener(listener)) {
                return;
            }

            const key = String(type);
            const listeners = this.__listeners.get(key) ?? [];
            const normalizedOptions = normalizeListenerOptions(options);
            if (
                listeners.some(
                    (entry) =>
                        entry.listener === listener &&
                        entry.capture === normalizedOptions.capture,
                )
            ) {
                return;
            }
            listeners.push({
                listener,
                capture: normalizedOptions.capture,
                once: normalizedOptions.once,
            });
            this.__listeners.set(key, listeners);

            if (hasCompletedEvent(this, key)) {
                replayCompletedEvent(this, key, listeners[listeners.length - 1]);
            }
        }

        removeEventListener(type, listener, options = false) {
            const key = String(type);
            const capture = normalizeListenerOptions(options).capture;
            const listeners = this.__listeners.get(key) ?? [];
            this.__listeners.set(
                key,
                listeners.filter(
                    (entry) =>
                        entry.listener !== listener || entry.capture !== capture,
                ),
            );
        }

        dispatchEvent(event) {
            const instance = event instanceof Event ? event : new Event(event?.type ?? event);
            instance.target = this;
            instance.__dispatchPath = buildEventPath(this);
            instance.__stopPropagation = false;
            instance.__stopImmediatePropagation = false;

            for (let index = instance.__dispatchPath.length - 1; index >= 1; index -= 1) {
                if (instance.__stopPropagation) {
                    break;
                }
                dispatchToTarget(
                    instance.__dispatchPath[index],
                    instance,
                    Event.CAPTURING_PHASE,
                );
            }

            if (!instance.__stopPropagation) {
                dispatchToTarget(this, instance, Event.AT_TARGET);
            }

            if (instance.bubbles && !instance.__stopPropagation) {
                for (let index = 1; index < instance.__dispatchPath.length; index += 1) {
                    if (instance.__stopPropagation) {
                        break;
                    }
                    dispatchToTarget(
                        instance.__dispatchPath[index],
                        instance,
                        Event.BUBBLING_PHASE,
                    );
                }
            }

            instance.currentTarget = null;
            instance.eventPhase = Event.NONE;
            return !instance.defaultPrevented;
        }
    }

    function normalizeListenerOptions(options) {
        if (typeof options === "boolean") {
            return { capture: options, once: false };
        }
        if (!options || typeof options !== "object") {
            return { capture: false, once: false };
        }
        return {
            capture: Boolean(options.capture),
            once: Boolean(options.once),
        };
    }

    function isEventListener(listener) {
        return typeof listener === "function" ||
            (
                listener &&
                typeof listener === "object" &&
                typeof listener.handleEvent === "function"
            );
    }

    function invokeEventListener(target, listener, event) {
        if (typeof listener === "function") {
            listener.call(target, event);
            return;
        }

        listener.handleEvent.call(listener, event);
    }

    function hasCompletedEvent(target, type) {
        return Boolean(
            target &&
            target.__romCompletedEvents &&
            target.__romCompletedEvents.has(String(type)),
        );
    }

    function createCompletedEvent(type) {
        const event = new Event(type);
        if (type === "pageshow") {
            event.persisted = false;
        }
        return event;
    }

    function replayCompletedEvent(target, type, entry) {
        if (!hasCompletedEvent(target, type)) {
            return;
        }

        if (
            String(type) === "DOMContentLoaded" &&
            (
                target?.__romSyntheticDomContentLoadedQueued ||
                target?.__romSuppressDomContentLoadedReplay
            )
        ) {
            return;
        }

        setTimeout(() => {
            Promise.resolve().then(() => {
                const listeners = target.__listeners?.get(String(type)) ?? [];
                if (!listeners.includes(entry)) {
                    return;
                }

                const event = createCompletedEvent(type);
                event.target = target;
                event.currentTarget = target;
                event.eventPhase = Event.AT_TARGET;
                invokeEventListener(target, entry.listener, event);
                if (entry.once) {
                    target.removeEventListener(type, entry.listener, {
                        capture: entry.capture,
                    });
                }
            });
        }, 0);
    }

    function buildEventPath(target) {
        const path = [target];
        let current = target?.parentNode ?? null;
        while (current) {
            path.push(current);
            current = current.parentNode ?? null;
        }
        return path;
    }

    function dispatchToTarget(target, event, phase) {
        event.currentTarget = target;
        event.eventPhase = phase;
        event.__stopImmediatePropagation = false;
        const listeners = target.__listeners?.get(event.type) ?? [];

        for (const entry of listeners.slice()) {
            if (!shouldInvokeListener(entry, phase)) {
                continue;
            }

            invokeEventListener(target, entry.listener, event);
            if (entry.once) {
                target.removeEventListener(event.type, entry.listener, {
                    capture: entry.capture,
                });
            }
            if (event.__stopImmediatePropagation) {
                break;
            }
        }
    }

    function shouldInvokeListener(entry, phase) {
        if (phase === Event.CAPTURING_PHASE) {
            return entry.capture;
        }
        if (phase === Event.BUBBLING_PHASE) {
            return !entry.capture;
        }
        return true;
    }

    function walk(root, visit) {
        for (const child of root.childNodes) {
            if (visit(child)) {
                return child;
            }
            const nested = walk(child, visit);
            if (nested) {
                return nested;
            }
        }
        return null;
    }

    function matchesSelector(node, selector) {
        if (!node || node.nodeType !== 1) {
            return false;
        }

        const parsed = parseSimpleSelector(selector);
        if (!parsed) {
            return false;
        }

        if (parsed.tagName !== null && parsed.tagName !== "*" && node.tagName.toLowerCase() !== parsed.tagName) {
            return false;
        }

        if (parsed.id !== null && node.id !== parsed.id) {
            return false;
        }

        const classNames = node.className.split(/\s+/).filter(Boolean);
        for (const className of parsed.classNames) {
            if (!classNames.includes(className)) {
                return false;
            }
        }

        for (const attribute of parsed.attributes) {
            if (!node.hasAttribute(attribute.name)) {
                return false;
            }

            if (attribute.value !== null && node.getAttribute(attribute.name) !== attribute.value) {
                return false;
            }
        }

        return true;
    }

    function parseSimpleSelector(selector) {
        const source = String(selector ?? "").trim();
        if (!source || /\s/.test(source)) {
            return null;
        }

        let index = 0;
        let tagName = null;
        let id = null;
        const classNames = [];
        const attributes = [];

        if (source[index] === "*") {
            tagName = "*";
            index += 1;
        } else if (isIdentifierStart(source[index])) {
            const identifier = readSelectorIdentifier(source, index);
            tagName = identifier.value.toLowerCase();
            index = identifier.nextIndex;
        }

        while (index < source.length) {
            const token = source[index];
            if (token === "#") {
                const identifier = readSelectorIdentifier(source, index + 1);
                if (!identifier.value) {
                    return null;
                }
                id = identifier.value;
                index = identifier.nextIndex;
                continue;
            }

            if (token === ".") {
                const identifier = readSelectorIdentifier(source, index + 1);
                if (!identifier.value) {
                    return null;
                }
                classNames.push(identifier.value);
                index = identifier.nextIndex;
                continue;
            }

            if (token === "[") {
                const attribute = readSelectorAttribute(source, index + 1);
                if (!attribute) {
                    return null;
                }
                attributes.push(attribute.attribute);
                index = attribute.nextIndex;
                continue;
            }

            return null;
        }

        return {
            tagName,
            id,
            classNames,
            attributes,
        };
    }

    function readSelectorIdentifier(source, startIndex) {
        let index = startIndex;
        while (index < source.length && isIdentifierCharacter(source[index])) {
            index += 1;
        }

        return {
            value: source.slice(startIndex, index),
            nextIndex: index,
        };
    }

    function readSelectorAttribute(source, startIndex) {
        let index = startIndex;
        while (index < source.length && /\s/.test(source[index])) {
            index += 1;
        }

        const nameIdentifier = readSelectorIdentifier(source, index);
        if (!nameIdentifier.value) {
            return null;
        }

        index = nameIdentifier.nextIndex;
        while (index < source.length && /\s/.test(source[index])) {
            index += 1;
        }

        let value = null;
        if (source[index] === "=") {
            index += 1;
            while (index < source.length && /\s/.test(source[index])) {
                index += 1;
            }

            if (source[index] === "\"" || source[index] === "'") {
                const quote = source[index];
                index += 1;
                const valueStart = index;
                while (index < source.length && source[index] !== quote) {
                    index += 1;
                }
                if (index >= source.length) {
                    return null;
                }
                value = source.slice(valueStart, index);
                index += 1;
            } else {
                const valueIdentifier = readSelectorIdentifier(source, index);
                if (!valueIdentifier.value) {
                    return null;
                }
                value = valueIdentifier.value;
                index = valueIdentifier.nextIndex;
            }

            while (index < source.length && /\s/.test(source[index])) {
                index += 1;
            }
        }

        if (source[index] !== "]") {
            return null;
        }

        return {
            attribute: {
                name: nameIdentifier.value,
                value,
            },
            nextIndex: index + 1,
        };
    }

    function isIdentifierStart(character) {
        return typeof character === "string" && /[A-Za-z_]/.test(character);
    }

    function isIdentifierCharacter(character) {
        return typeof character === "string" && /[A-Za-z0-9_-]/.test(character);
    }

    function querySelectorFrom(root, selector) {
        return walk(root, (node) => matchesSelector(node, selector));
    }

    function querySelectorAllFrom(root, selector) {
        const matches = [];

        walk(root, (node) => {
            if (matchesSelector(node, selector)) {
                matches.push(node);
            }
            return false;
        });

        return matches;
    }

    function createCanvasContext(kind) {
        return {
            kind,
            canvas: null,
            fillStyle: "#000000",
            font: "10px sans-serif",
            textBaseline: "alphabetic",
            globalCompositeOperation: "source-over",
            fillRect() {},
            clearRect() {},
            beginPath() {},
            rect() {},
            arc() {},
            closePath() {},
            fill() {},
            fillText() {},
            strokeText() {},
            drawImage() {},
            isPointInPath() {
                return false;
            },
            getImageData(x = 0, y = 0, width = 1, height = 1) {
                return {
                    data: new Uint8ClampedArray(width * height * 4),
                    width,
                    height,
                };
            },
            measureText(text = "") {
                return { width: String(text).length * 7.25 };
            },
        };
    }

    function createStyleDeclaration() {
        return {
            setProperty(name, value) {
                this[name] = String(value);
            },
            getPropertyValue(name) {
                return this[name] ?? "";
            },
            removeProperty(name) {
                const previous = this[name] ?? "";
                delete this[name];
                return previous;
            },
        };
    }

    function defineReadOnly(target, key, value) {
        Object.defineProperty(target, key, {
            configurable: true,
            enumerable: true,
            writable: false,
            value,
        });
    }
