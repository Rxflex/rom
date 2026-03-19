    class MessageEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.data = init.data ?? null;
            this.origin = String(init.origin ?? "");
            this.lastEventId = String(init.lastEventId ?? "");
            this.source = init.source ?? null;
            this.ports = Array.isArray(init.ports) ? init.ports.slice() : [];
        }
    }

    class MessagePort extends EventTarget {
        constructor() {
            super();
            this.__peer = null;
            this.__closed = false;
            this.__started = false;
            this.__messageQueue = [];
            this.__onmessage = null;
        }

        get onmessage() {
            return this.__onmessage;
        }

        set onmessage(listener) {
            this.__onmessage = typeof listener === "function" ? listener : null;
            if (this.__onmessage) {
                this.start();
            }
        }

        postMessage(data, transfer = []) {
            if (!this.__peer || this.__closed || this.__peer.__closed) {
                return;
            }
            validateTransferList(transfer);

            const event = new MessageEvent("message", {
                data: structuredClone(data),
                origin: location.origin,
                source: this,
                ports: [],
            });

            queueMicrotask(() => {
                if (!this.__peer || this.__peer.__closed) {
                    return;
                }

                this.__peer.__enqueueMessage(event);
            });
        }

        start() {
            if (this.__closed) {
                return;
            }

            this.__started = true;
            this.__drainMessageQueue();
        }

        close() {
            this.__closed = true;
            this.__messageQueue = [];
        }

        __enqueueMessage(event) {
            if (this.__closed) {
                return;
            }

            this.__messageQueue.push(event);
            if (this.__started) {
                this.__drainMessageQueue();
            }
        }

        __drainMessageQueue() {
            if (!this.__started || this.__closed) {
                return;
            }

            while (this.__messageQueue.length > 0) {
                const event = this.__messageQueue.shift();
                if (typeof this.__onmessage === "function") {
                    this.__onmessage(event);
                }

                this.dispatchEvent(event);
            }
        }
    }

    class MessageChannel {
        constructor() {
            this.port1 = new MessagePort();
            this.port2 = new MessagePort();
            this.port1.__peer = this.port2;
            this.port2.__peer = this.port1;
        }
    }

    const broadcastChannelRegistry = new Map();

    class BroadcastChannel extends EventTarget {
        constructor(name) {
            super();
            this.name = String(name);
            this.onmessage = null;
            this.onmessageerror = null;
            this.__closed = false;
            this.__registryKey = `${location.origin}::${this.name}`;

            const entries = broadcastChannelRegistry.get(this.__registryKey) ?? new Set();
            entries.add(this);
            broadcastChannelRegistry.set(this.__registryKey, entries);
        }

        postMessage(data) {
            if (this.__closed) {
                return;
            }

            const peers = broadcastChannelRegistry.get(this.__registryKey) ?? new Set();
            const payload = structuredClone(data);

            for (const peer of peers) {
                if (peer === this || peer.__closed) {
                    continue;
                }

                const event = new MessageEvent("message", {
                    data: structuredClone(payload),
                    origin: location.origin,
                    source: null,
                    ports: [],
                });

                queueMicrotask(() => {
                    if (peer.__closed) {
                        return;
                    }

                    if (typeof peer.onmessage === "function") {
                        peer.onmessage(event);
                    }

                    peer.dispatchEvent(event);
                });
            }
        }

        close() {
            if (this.__closed) {
                return;
            }

            this.__closed = true;
            const peers = broadcastChannelRegistry.get(this.__registryKey);
            if (!peers) {
                return;
            }

            peers.delete(this);
            if (peers.size === 0) {
                broadcastChannelRegistry.delete(this.__registryKey);
            }
        }
    }

    class Worker extends EventTarget {
        constructor(specifier) {
            super();
            this.onmessage = null;
            this.onerror = null;
            this.__terminated = false;
            this.__ready = false;
            this.__failed = false;
            this.__scheduledTimers = new Set();
            this.__url = new URL(String(specifier), location.href).href;
            this.__scope = createWorkerScope(this, this.__url);
            queueMicrotask(() => {
                if (this.__terminated) {
                    return;
                }

                let source = "";
                try {
                    source = resolveWorkerSource(this.__url);
                } catch (error) {
                    this.__failed = true;
                    dispatchWorkerError(this, error);
                    return;
                }

                try {
                    executeWorkerSource(this.__scope, source);
                    this.__ready = true;
                } catch (_error) {
                    this.__failed = true;
                }
            });
        }

        postMessage(data, transfer = []) {
            if (this.__terminated) {
                return;
            }
            validateTransferList(transfer);

            const event = new MessageEvent("message", {
                data: structuredClone(data),
                origin: location.origin,
                source: null,
                ports: [],
            });

            queueMicrotask(() => {
                if (this.__terminated || this.__failed || !this.__ready) {
                    return;
                }

                try {
                    if (typeof this.__scope.onmessage === "function") {
                        this.__scope.onmessage(event);
                    }

                    this.__scope.dispatchEvent(event);
                } catch (error) {
                    dispatchWorkerError(this, error);
                }
            });
        }

        terminate() {
            terminateWorker(this);
        }
    }

    function createWorkerScope(worker, workerUrl) {
        class DedicatedWorkerGlobalScope extends EventTarget {}

        const scope = new DedicatedWorkerGlobalScope();
        scope.__worker = worker;
        scope.self = scope;
        scope.globalThis = scope;
        scope.onmessage = null;
        scope.close = () => {
            terminateWorker(worker);
        };
        scope.postMessage = (data, transfer = []) => {
            if (worker.__terminated) {
                return;
            }
            validateTransferList(transfer);

            const event = new MessageEvent("message", {
                data: structuredClone(data),
                origin: new URL(workerUrl).origin,
                source: null,
                ports: [],
            });

            queueMicrotask(() => {
                if (worker.__terminated) {
                    return;
                }

                if (typeof worker.onmessage === "function") {
                    worker.onmessage(event);
                }

                worker.dispatchEvent(event);
            });
        };
        scope.importScripts = (...specifiers) => {
            for (const specifier of specifiers) {
                const sourceUrl = new URL(String(specifier), workerUrl).href;
                const source = resolveWorkerSource(sourceUrl);
                executeWorkerSource(scope, source);
            }
        };
        scope.structuredClone = structuredClone;
        scope.MessageEvent = MessageEvent;
        scope.MessagePort = MessagePort;
        scope.MessageChannel = MessageChannel;
        scope.BroadcastChannel = BroadcastChannel;
        scope.EventSource = EventSource;
        scope.WebSocket = WebSocket;
        scope.CloseEvent = CloseEvent;
        scope.URL = URL;
        scope.URLSearchParams = URLSearchParams;
        scope.URLPattern = URLPattern;
        scope.Blob = Blob;
        scope.File = File;
        scope.FileReader = FileReader;
        scope.DOMParser = DOMParser;
        scope.Headers = Headers;
        scope.Request = Request;
        scope.Response = Response;
        scope.fetch = fetch;
        scope.AbortController = AbortController;
        scope.AbortSignal = AbortSignal;
        scope.FormData = FormData;
        scope.TextEncoder = TextEncoder;
        scope.TextDecoder = TextDecoder;
        scope.Permissions = Permissions;
        scope.PermissionStatus = PermissionStatus;
        scope.NavigatorUAData = NavigatorUAData;
        scope.MediaDevices = MediaDevices;
        scope.MediaDeviceInfo = MediaDeviceInfo;
        scope.InputDeviceInfo = InputDeviceInfo;
        scope.MediaStream = MediaStream;
        scope.MediaStreamTrack = MediaStreamTrack;
        scope.Plugin = Plugin;
        scope.PluginArray = PluginArray;
        scope.MimeType = MimeType;
        scope.MimeTypeArray = MimeTypeArray;
        scope.MutationObserver = MutationObserver;
        scope.ResizeObserver = ResizeObserver;
        scope.IntersectionObserver = IntersectionObserver;
        scope.crypto = crypto;
        scope.performance = performance;
        scope.console = console;
        scope.setTimeout = (...args) => scheduleWorkerTimer(worker, false, ...args);
        scope.clearTimeout = (timerId) => clearWorkerTimer(worker, timerId);
        scope.setInterval = (...args) => scheduleWorkerTimer(worker, true, ...args);
        scope.clearInterval = (timerId) => clearWorkerTimer(worker, timerId);
        scope.queueMicrotask = (callback) => {
            queueMicrotask(() => {
                if (worker.__terminated) {
                    return;
                }
                callback();
            });
        };
        scope.navigator = navigator;
        scope.location = new URL(workerUrl);
        scope.origin = scope.location.origin;
        return scope;
    }

    function scheduleWorkerTimer(worker, repeat, callback, delay = 0, ...args) {
        let timerId = null;
        const runner = () => {
            if (worker.__terminated) {
                worker.__scheduledTimers.delete(timerId);
                return;
            }

            callback(...args);

            if (!repeat) {
                worker.__scheduledTimers.delete(timerId);
            }
        };
        timerId = repeat ? setInterval(runner, delay) : setTimeout(runner, delay);
        worker.__scheduledTimers.add(timerId);
        return timerId;
    }

    function clearWorkerTimer(worker, timerId) {
        clearTimeout(timerId);
        clearInterval(timerId);
        worker.__scheduledTimers.delete(timerId);
    }

    function terminateWorker(worker) {
        if (worker.__terminated) {
            return;
        }

        worker.__terminated = true;
        for (const timerId of worker.__scheduledTimers) {
            clearTimeout(timerId);
            clearInterval(timerId);
        }
        worker.__scheduledTimers.clear();
    }

    function resolveWorkerSource(workerUrl) {
        if (workerUrl.startsWith("blob:")) {
            const entry = objectUrlRegistry.get(workerUrl);
            if (!entry) {
                throw new TypeError("Failed to construct 'Worker': script not found.");
            }
            return decodeBytes(entry.bytes);
        }

        if (workerUrl.startsWith("data:")) {
            return decodeDataUrl(workerUrl);
        }

        const response = JSON.parse(
            g.__rom_fetch_sync(
                JSON.stringify({
                    url: workerUrl,
                    method: "GET",
                    headers: [],
                    body: [],
                }),
            ),
        );

        if (response.status < 200 || response.status >= 300) {
            throw new TypeError("Failed to construct 'Worker': unable to load script.");
        }

        return decodeBytes(response.body);
    }

    function executeWorkerSource(scope, source) {
        try {
            const runner = new Function(
                "self",
                `
                with (self) {
                    ${source}
                }
                `,
            );
            runner(scope);
        } catch (error) {
            dispatchWorkerError(scope.__worker ?? null, error);
            throw error;
        }
    }

    function dispatchWorkerError(worker, error) {
        if (!worker) {
            return;
        }

        const event = new Event("error");
        event.error = error;
        event.target = worker;
        event.currentTarget = worker;

        if (typeof worker.onerror === "function") {
            worker.onerror(event);
        }

        worker.dispatchEvent(event);
    }

    function decodeDataUrl(workerUrl) {
        const separatorIndex = workerUrl.indexOf(",");
        if (separatorIndex === -1) {
            throw new TypeError("Failed to construct 'Worker': invalid data URL.");
        }

        const metadata = workerUrl.slice(5, separatorIndex);
        const body = workerUrl.slice(separatorIndex + 1);

        if (metadata.endsWith(";base64")) {
            return decodeBase64(body);
        }

        return decodeURIComponent(body);
    }

    function decodeBase64(value) {
        const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        const clean = String(value).replace(/=+$/, "");
        const bytes = [];
        let buffer = 0;
        let bits = 0;

        for (const char of clean) {
            const index = alphabet.indexOf(char);
            if (index === -1) {
                continue;
            }

            buffer = (buffer << 6) | index;
            bits += 6;

            if (bits >= 8) {
                bits -= 8;
                bytes.push((buffer >> bits) & 0xff);
            }
        }

        return decodeBytes(bytes);
    }

    function structuredClone(value, options = undefined) {
        validateStructuredCloneOptions(options);
        return cloneStructuredValue(value, new Map());
    }

    function validateStructuredCloneOptions(options) {
        if (options === undefined || options === null) {
            return;
        }

        if (typeof options !== "object") {
            throw new TypeError("The provided value is not a valid structured clone options dictionary.");
        }

        validateTransferList(options.transfer ?? []);
    }

    function validateTransferList(transfer) {
        if (transfer === undefined || transfer === null) {
            return;
        }

        const values = Array.isArray(transfer) ? transfer : [transfer];
        if (values.length === 0) {
            return;
        }

        throw new TypeError("DataCloneError");
    }

    function cloneStructuredValue(value, seen) {
        if (value === null || typeof value !== "object") {
            if (typeof value === "function" || typeof value === "symbol") {
                throw new TypeError("DataCloneError");
            }
            return value;
        }

        if (seen.has(value)) {
            return seen.get(value);
        }

        if (value instanceof Date) {
            return new Date(value.getTime());
        }

        if (value instanceof Error) {
            const clone = new value.constructor(value.message);
            seen.set(value, clone);
            clone.name = value.name;

            if ("stack" in value && value.stack !== undefined) {
                clone.stack = String(value.stack);
            }

            for (const key of Object.keys(value)) {
                clone[key] = cloneStructuredValue(value[key], seen);
            }

            return clone;
        }

        if (value instanceof RegExp) {
            return new RegExp(value.source, value.flags);
        }

        if (value instanceof ArrayBuffer) {
            return value.slice(0);
        }

        if (ArrayBuffer.isView(value)) {
            const buffer = value.buffer.slice(
                value.byteOffset,
                value.byteOffset + value.byteLength,
            );
            return new value.constructor(buffer);
        }

        if (value instanceof Blob) {
            return new Blob([value.__bytes.slice()], { type: value.type });
        }

        if (value instanceof File) {
            return new File([value.__bytes.slice()], value.name, {
                type: value.type,
                lastModified: value.lastModified,
            });
        }

        if (value instanceof Map) {
            const clone = new Map();
            seen.set(value, clone);
            for (const [key, entryValue] of value.entries()) {
                clone.set(
                    cloneStructuredValue(key, seen),
                    cloneStructuredValue(entryValue, seen),
                );
            }
            return clone;
        }

        if (value instanceof Set) {
            const clone = new Set();
            seen.set(value, clone);
            for (const entryValue of value.values()) {
                clone.add(cloneStructuredValue(entryValue, seen));
            }
            return clone;
        }

        if (Array.isArray(value)) {
            const clone = [];
            seen.set(value, clone);
            for (const entryValue of value) {
                clone.push(cloneStructuredValue(entryValue, seen));
            }
            return clone;
        }

        const prototype = Object.getPrototypeOf(value);
        if (prototype !== Object.prototype && prototype !== null) {
            throw new TypeError("DataCloneError");
        }

        const clone = {};
        seen.set(value, clone);
        for (const key of Object.keys(value)) {
            clone[key] = cloneStructuredValue(value[key], seen);
        }
        return clone;
    }
