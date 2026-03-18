(() => {
    const g = globalThis;
    const config = g.__rom_config ?? {};
    const navigatorConfig = config.navigator ?? {};
    const locationConfig = config.location ?? {};
    const nowBase = Date.now();

    class Event {
        constructor(type, init = {}) {
            this.type = String(type ?? "");
            this.bubbles = Boolean(init.bubbles);
            this.cancelable = Boolean(init.cancelable);
            this.defaultPrevented = false;
            this.target = null;
            this.currentTarget = null;
        }

        preventDefault() {
            if (this.cancelable) {
                this.defaultPrevented = true;
            }
        }
    }

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

        addEventListener(type, listener) {
            if (typeof listener !== "function") {
                return;
            }

            const key = String(type);
            const listeners = this.__listeners.get(key) ?? [];
            listeners.push(listener);
            this.__listeners.set(key, listeners);
        }

        removeEventListener(type, listener) {
            const key = String(type);
            const listeners = this.__listeners.get(key) ?? [];
            this.__listeners.set(
                key,
                listeners.filter((entry) => entry !== listener),
            );
        }

        dispatchEvent(event) {
            const instance = event instanceof Event ? event : new Event(event?.type ?? event);
            instance.target = this;
            instance.currentTarget = this;
            const listeners = this.__listeners.get(instance.type) ?? [];
            for (const listener of listeners) {
                listener.call(this, instance);
            }
            return !instance.defaultPrevented;
        }
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
        if (selector.startsWith("#")) {
            return node.id === selector.slice(1);
        }
        if (selector.startsWith(".")) {
            return node.className.split(/\s+/).filter(Boolean).includes(selector.slice(1));
        }
        return node.tagName.toLowerCase() === selector.toLowerCase();
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
