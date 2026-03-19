    const location = {
        href: locationConfig.href ?? "https://rom.local/",
        origin: locationConfig.origin ?? "https://rom.local",
        protocol: locationConfig.protocol ?? "https:",
        host: locationConfig.host ?? "rom.local",
        hostname: locationConfig.hostname ?? "rom.local",
        pathname: locationConfig.pathname ?? "/",
        search: locationConfig.search ?? "",
        hash: locationConfig.hash ?? "",
        assign(nextHref) {
            navigateLocation("push", nextHref);
        },
        replace(nextHref) {
            navigateLocation("replace", nextHref);
        },
        reload() {},
        toString() {
            return this.href;
        },
    };

    function applyLocationHref(targetLocation, nextHref) {
        const parsed = new URL(String(nextHref), targetLocation.href);
        targetLocation.href = parsed.href;
        targetLocation.origin = parsed.origin;
        targetLocation.protocol = parsed.protocol;
        targetLocation.host = parsed.host;
        targetLocation.hostname = parsed.hostname;
        targetLocation.pathname = parsed.pathname;
        targetLocation.search = parsed.search;
        targetLocation.hash = parsed.hash;
    }

    const navigator = createNavigator(navigatorConfig);

    const historyEntries = [{ href: location.href, state: null }];
    let currentHistoryIndex = 0;

    const history = {
        length: historyEntries.length,
        state: null,
        back() {
            traverseHistory(-1);
        },
        forward() {
            traverseHistory(1);
        },
        go(delta = 0) {
            const numericDelta = Number(delta);
            if (!Number.isFinite(numericDelta)) {
                return;
            }

            traverseHistory(Math.trunc(numericDelta));
        },
        pushState(state, _title, nextHref) {
            updateHistoryEntry("push", state, nextHref);
        },
        replaceState(state, _title, nextHref) {
            updateHistoryEntry("replace", state, nextHref);
        },
    };

    function cloneHistoryState(value) {
        if (typeof structuredClone === "function") {
            return structuredClone(value);
        }

        return value;
    }

    function createDomException(name, message) {
        const error = new Error(message);
        error.name = name;
        return error;
    }

    function resolveHistoryHref(nextHref) {
        if (nextHref === undefined || nextHref === null || nextHref === "") {
            return location.href;
        }

        const resolved = new URL(String(nextHref), location.href);
        if (resolved.origin !== location.origin) {
            throw createDomException(
                "SecurityError",
                "History state URLs must stay on the current origin.",
            );
        }

        return resolved.href;
    }

    function syncHistoryState(entry) {
        applyLocationHref(location, entry.href);
        history.length = historyEntries.length;
        history.state = cloneHistoryState(entry.state);
    }

    function updateHistoryEntry(mode, state, nextHref) {
        const entry = {
            href: resolveHistoryHref(nextHref),
            state: cloneHistoryState(state),
        };

        if (mode === "replace") {
            historyEntries[currentHistoryIndex] = entry;
        } else {
            historyEntries.splice(currentHistoryIndex + 1);
            historyEntries.push(entry);
            currentHistoryIndex = historyEntries.length - 1;
        }

        syncHistoryState(entry);
    }

    function navigateLocation(mode, nextHref) {
        const nextState = mode === "replace"
            ? historyEntries[currentHistoryIndex]?.state ?? null
            : null;
        updateHistoryEntry(mode, nextState, nextHref);
    }

    function traverseHistory(delta) {
        if (!delta) {
            return;
        }

        const nextIndex = currentHistoryIndex + delta;
        if (nextIndex < 0 || nextIndex >= historyEntries.length) {
            return;
        }

        currentHistoryIndex = nextIndex;
        const entry = historyEntries[currentHistoryIndex];
        syncHistoryState(entry);
        dispatchWindowEvent(
            new PopStateEvent("popstate", {
                state: cloneHistoryState(entry.state),
            }),
        );
    }

    const viewport = createViewportState();
    const screen = createScreen(viewport);

    let nextTimerId = 1;
    const timers = new Map();

    function registerTimer(callback, interval, delay, args) {
        const timerId = nextTimerId++;
        timers.set(timerId, {
            callback,
            interval,
            args,
            delay: normalizeTimerDelay(delay),
        });

        queueMicrotask(() => runTimer(timerId));

        return timerId;
    }

    function runTimer(timerId) {
        const timer = timers.get(timerId);
        if (!timer || typeof timer.callback !== "function") {
            return;
        }

        if (timer.delay > 0) {
            g.__rom_sleep_ms(timer.delay);
        }

        if (timers.get(timerId) !== timer) {
            return;
        }

        timer.callback(...timer.args);

        if (!timer.interval) {
            timers.delete(timerId);
            return;
        }

        if (timers.get(timerId) === timer) {
            queueMicrotask(() => runTimer(timerId));
        }
    }

    function clearTimer(timerId) {
        timers.delete(timerId);
    }

    function normalizeTimerDelay(delay) {
        const numeric = Number(delay);
        if (!Number.isFinite(numeric) || numeric <= 0) {
            return 0;
        }

        return Math.min(2147483647, Math.trunc(numeric));
    }

    const performanceEntries = [];
    const performanceObservers = new Set();

    class PerformanceEntry {
        constructor(name, entryType, startTime, duration, detail = null) {
            this.name = String(name);
            this.entryType = String(entryType);
            this.startTime = Number(startTime) || 0;
            this.duration = Number(duration) || 0;
            this.detail = detail ?? null;
        }

        toJSON() {
            return {
                name: this.name,
                entryType: this.entryType,
                startTime: this.startTime,
                duration: this.duration,
                detail: this.detail,
            };
        }
    }

    class PerformanceObserverEntryList {
        constructor(entries) {
            this.__entries = entries.slice().sort(comparePerformanceEntries);
        }

        getEntries() {
            return this.__entries.slice();
        }

        getEntriesByType(type) {
            const normalizedType = String(type);
            return this.__entries.filter((entry) => entry.entryType === normalizedType);
        }

        getEntriesByName(name, type = undefined) {
            const normalizedName = String(name);
            const normalizedType = type === undefined ? null : String(type);
            return this.__entries.filter(
                (entry) =>
                    entry.name === normalizedName &&
                    (normalizedType === null || entry.entryType === normalizedType),
            );
        }
    }

    class PerformanceObserver {
        constructor(callback) {
            this.callback = typeof callback === "function" ? callback : () => {};
            this.__entryTypes = [];
            this.__records = [];
            this.__scheduled = false;
            performanceObservers.add(this);
        }

        observe(options = {}) {
            if (!options || typeof options !== "object") {
                throw new TypeError("Failed to observe performance timeline.");
            }

            let entryTypes = [];
            if (Array.isArray(options.entryTypes)) {
                entryTypes = options.entryTypes.map(String);
            } else if (options.type !== undefined) {
                entryTypes = [String(options.type)];
            }

            if (!entryTypes.length) {
                throw new TypeError("PerformanceObserver requires entryTypes or type.");
            }

            this.__entryTypes = entryTypes.filter(isSupportedPerformanceEntryType);
        }

        disconnect() {
            this.__entryTypes = [];
            this.__records = [];
            this.__scheduled = false;
        }

        takeRecords() {
            const records = this.__records.slice().sort(comparePerformanceEntries);
            this.__records = [];
            return records;
        }

        static get supportedEntryTypes() {
            return ["mark", "measure"];
        }
    }

    function comparePerformanceEntries(left, right) {
        if (left.startTime !== right.startTime) {
            return left.startTime - right.startTime;
        }
        return performanceEntries.indexOf(left) - performanceEntries.indexOf(right);
    }

    function isSupportedPerformanceEntryType(type) {
        return type === "mark" || type === "measure";
    }

    function clonePerformanceDetail(detail) {
        if (detail === undefined) {
            return null;
        }

        if (typeof structuredClone === "function") {
            return structuredClone(detail);
        }

        return detail;
    }

    function addPerformanceEntry(entry) {
        performanceEntries.push(entry);
        queuePerformanceObservers(entry);
        return entry;
    }

    function queuePerformanceObservers(entry) {
        for (const observer of performanceObservers) {
            if (!observer.__entryTypes.includes(entry.entryType)) {
                continue;
            }

            observer.__records.push(entry);
            if (observer.__scheduled) {
                continue;
            }

            observer.__scheduled = true;
            queueMicrotask(() => {
                observer.__scheduled = false;
                if (!observer.__records.length || !observer.__entryTypes.length) {
                    observer.__records = [];
                    return;
                }

                observer.callback(
                    new PerformanceObserverEntryList(observer.takeRecords()),
                    observer,
                );
            });
        }
    }

    function getPerformanceEntries(type = null, name = null) {
        return performanceEntries
            .filter(
                (entry) =>
                    (type === null || entry.entryType === type) &&
                    (name === null || entry.name === name),
            )
            .slice()
            .sort(comparePerformanceEntries);
    }

    function clearPerformanceEntries(type, name = null) {
        for (let index = performanceEntries.length - 1; index >= 0; index -= 1) {
            const entry = performanceEntries[index];
            if (entry.entryType !== type) {
                continue;
            }
            if (name !== null && entry.name !== name) {
                continue;
            }
            performanceEntries.splice(index, 1);
        }
    }

    function findLatestPerformanceMark(name) {
        const normalizedName = String(name);
        for (let index = performanceEntries.length - 1; index >= 0; index -= 1) {
            const entry = performanceEntries[index];
            if (entry.entryType === "mark" && entry.name === normalizedName) {
                return entry;
            }
        }

        throw createDomException(
            "SyntaxError",
            `The mark '${normalizedName}' does not exist.`,
        );
    }

    function resolvePerformanceTimestamp(reference, fallbackStartTime = null) {
        if (reference === undefined) {
            return fallbackStartTime === null ? performance.now() : fallbackStartTime;
        }

        if (typeof reference === "number") {
            return Number(reference);
        }

        if (typeof reference === "string") {
            return findLatestPerformanceMark(reference).startTime;
        }

        throw new TypeError("Invalid performance timestamp reference.");
    }

    const performance = {
        timeOrigin: nowBase,
        now() {
            return Date.now() - nowBase;
        },
        mark(name, options = {}) {
            const normalizedName = String(name);
            const normalizedOptions =
                options && typeof options === "object" ? options : {};
            const startTime = resolvePerformanceTimestamp(
                normalizedOptions.startTime,
                performance.now(),
            );
            const entry = new PerformanceEntry(
                normalizedName,
                "mark",
                startTime,
                0,
                clonePerformanceDetail(normalizedOptions.detail),
            );
            return addPerformanceEntry(entry);
        },
        measure(name, startOrOptions = undefined, endMark = undefined) {
            const normalizedName = String(name);
            let startTime = 0;
            let endTime = performance.now();
            let detail = null;

            if (
                startOrOptions &&
                typeof startOrOptions === "object" &&
                !Array.isArray(startOrOptions)
            ) {
                const options = startOrOptions;
                detail = clonePerformanceDetail(options.detail);
                if (options.duration !== undefined) {
                    const duration = Number(options.duration);
                    if (options.start !== undefined) {
                        startTime = resolvePerformanceTimestamp(options.start);
                        endTime = startTime + duration;
                    } else if (options.end !== undefined) {
                        endTime = resolvePerformanceTimestamp(options.end);
                        startTime = endTime - duration;
                    } else {
                        startTime = performance.now();
                        endTime = startTime + duration;
                    }
                } else {
                    startTime = resolvePerformanceTimestamp(options.start, 0);
                    endTime = resolvePerformanceTimestamp(options.end, performance.now());
                }
            } else {
                startTime = resolvePerformanceTimestamp(startOrOptions, 0);
                endTime = resolvePerformanceTimestamp(endMark, performance.now());
            }

            const entry = new PerformanceEntry(
                normalizedName,
                "measure",
                startTime,
                Math.max(0, endTime - startTime),
                detail,
            );
            return addPerformanceEntry(entry);
        },
        getEntries() {
            return getPerformanceEntries();
        },
        getEntriesByType(type) {
            return getPerformanceEntries(String(type));
        },
        getEntriesByName(name, type = undefined) {
            const normalizedType = type === undefined ? null : String(type);
            return getPerformanceEntries(normalizedType, String(name));
        },
        clearMarks(name = undefined) {
            clearPerformanceEntries(
                "mark",
                name === undefined ? null : String(name),
            );
        },
        clearMeasures(name = undefined) {
            clearPerformanceEntries(
                "measure",
                name === undefined ? null : String(name),
            );
        },
    };

    const crypto = createCrypto();

    const visualViewport = createVisualViewport(viewport);
    const mediaQueryList = createMatchMedia(viewport);

    const audioContextFactory = function AudioContext() {
        return {
            sampleRate: 44100,
            state: "running",
            baseLatency: 0.01,
            destination: {},
            createOscillator() {
                return {
                    type: "sine",
                    frequency: { value: 0 },
                    connect() {},
                    start() {},
                    stop() {},
                };
            },
            createDynamicsCompressor() {
                return {
                    threshold: { value: 0 },
                    knee: { value: 0 },
                    ratio: { value: 0 },
                    attack: { value: 0 },
                    release: { value: 0 },
                    connect() {},
                };
            },
            createAnalyser() {
                return {
                    connect() {},
                    getFloatFrequencyData(target) {
                        target.fill(0);
                    },
                };
            },
            close() {},
        };
    };

    class OfflineAudioContext {
        constructor(_channels = 1, length = 5000, sampleRate = 44100) {
            this.length = length;
            this.sampleRate = sampleRate;
            this.state = "suspended";
            this.destination = {};
            this.oncomplete = null;
        }

        createOscillator() {
            return {
                type: "sine",
                frequency: { value: 0 },
                connect() {},
                start() {},
                stop() {},
            };
        }

        createDynamicsCompressor() {
            return {
                threshold: { value: 0 },
                knee: { value: 0 },
                ratio: { value: 0 },
                attack: { value: 0 },
                release: { value: 0 },
                connect() {},
            };
        }

        startRendering() {
            this.state = "running";

            const renderedBuffer = {
                getChannelData: () => new Float32Array(this.length),
            };

            Promise.resolve().then(() => {
                this.state = "closed";
                if (typeof this.oncomplete === "function") {
                    this.oncomplete({ renderedBuffer });
                }
            });

            return Promise.resolve(renderedBuffer);
        }
    }

    const console = {
        log: (...args) => g.__rom_console_log(args.map(String).join(" ")),
        warn: (...args) => g.__rom_console_warn(args.map(String).join(" ")),
        error: (...args) => g.__rom_console_error(args.map(String).join(" ")),
    };

    const CSS = {
        supports() {
            return false;
        },
    };

    class PopStateEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.state = Object.prototype.hasOwnProperty.call(init, "state")
                ? init.state
                : null;
        }
    }

    class Image {}

    class Audio {
        constructor() {
            this.currentTime = 0;
        }
    }

    class HTMLButtonElement extends Element {}

    class CompositionEvent extends Event {}

    function dispatchWindowEvent(event) {
        const instance = event instanceof Event ? event : new Event(event?.type ?? event);
        instance.target = g;
        instance.currentTarget = g;
        instance.eventPhase = Event.AT_TARGET;
        instance.__dispatchPath = [g];
        instance.__stopPropagation = false;
        instance.__stopImmediatePropagation = false;

        const handlerName = `on${instance.type}`;
        if (typeof g[handlerName] === "function") {
            g[handlerName].call(g, instance);
        }

        if (!instance.__stopImmediatePropagation) {
            const listeners = g.__listeners?.get(instance.type) ?? [];
            for (const entry of listeners.slice()) {
                entry.listener.call(g, instance);
                if (entry.once) {
                    g.removeEventListener(instance.type, entry.listener, {
                        capture: entry.capture,
                    });
                }
                if (instance.__stopImmediatePropagation) {
                    break;
                }
            }
        }

        instance.currentTarget = null;
        instance.eventPhase = Event.NONE;
        return !instance.defaultPrevented;
    }

    g.window = g;
    g.self = g;
    g.top = g;
    g.parent = g;
    g.__listeners = new Map();
    g.addEventListener = EventTarget.prototype.addEventListener.bind(g);
    g.removeEventListener = EventTarget.prototype.removeEventListener.bind(g);
    g.dispatchEvent = (event) => dispatchWindowEvent(event);
    g.onpopstate = null;
    g.document = document;
    g.navigator = navigator;
    g.location = location;
    g.history = history;
    g.screen = screen;
    g.innerWidth = viewport.innerWidth;
    g.innerHeight = viewport.innerHeight;
    g.outerWidth = viewport.outerWidth;
    g.outerHeight = viewport.outerHeight;
    g.devicePixelRatio = viewport.devicePixelRatio;
    g.visualViewport = visualViewport;
    g.localStorage = new Storage();
    g.sessionStorage = new Storage();
    g.performance = performance;
    g.crypto = crypto;
    g.CryptoKey = CryptoKey;
    g.SubtleCrypto = SubtleCrypto;
    g.console = console;
    g.CSS = CSS;
    g.Image = Image;
    g.Audio = Audio;
    g.HTMLButtonElement = HTMLButtonElement;
    g.CompositionEvent = CompositionEvent;
    g.PopStateEvent = PopStateEvent;
    g.MessageChannel = MessageChannel;
    g.MessagePort = MessagePort;
    g.MessageEvent = MessageEvent;
    g.BroadcastChannel = BroadcastChannel;
    g.Worker = Worker;
    g.EventSource = EventSource;
    g.WebSocket = WebSocket;
    g.CloseEvent = CloseEvent;
    g.structuredClone = structuredClone;
    g.matchMedia = mediaQueryList;
    g.TextEncoder = textEncoderFactory;
    g.TextDecoder = textDecoderFactory;
    g.setTimeout = (callback, delay = 0, ...args) => registerTimer(callback, false, delay, args);
    g.clearTimeout = clearTimer;
    g.setInterval = (callback, delay = 0, ...args) => registerTimer(callback, true, delay, args);
    g.clearInterval = clearTimer;
    g.queueMicrotask = (callback) =>
        Promise.resolve().then(() => {
            if (typeof callback === "function") {
                callback();
            }
        });
    g.requestAnimationFrame = (callback) =>
        registerTimer(() => callback(performance.now()), false, []);
    g.cancelAnimationFrame = clearTimer;
    g.fetch = fetch;
    g.Headers = Headers;
    g.Request = Request;
    g.Response = Response;
    g.URL = URL;
    g.URLSearchParams = URLSearchParams;
    g.URLPattern = URLPattern;
    g.AbortController = AbortController;
    g.AbortSignal = AbortSignal;
    g.Blob = Blob;
    g.File = File;
    g.FileReader = FileReader;
    g.FormData = FormData;
    g.ReadableStream = ReadableStream;
    g.Event = Event;
    g.CustomEvent = CustomEvent;
    g.EventTarget = EventTarget;
    g.Node = Node;
    g.Element = Element;
    g.HTMLElement = Element;
    g.HTMLCanvasElement = HTMLCanvasElement;
    g.Text = Text;
    g.Document = Document;
    g.DOMParser = DOMParser;
    g.Permissions = Permissions;
    g.PermissionStatus = PermissionStatus;
    g.NavigatorUAData = NavigatorUAData;
    g.MediaDevices = MediaDevices;
    g.MediaDeviceInfo = MediaDeviceInfo;
    g.InputDeviceInfo = InputDeviceInfo;
    g.ScreenOrientation = ScreenOrientation;
    g.VisualViewport = VisualViewport;
    g.MediaQueryList = MediaQueryList;
    g.MediaStream = MediaStream;
    g.MediaStreamTrack = MediaStreamTrack;
    g.Plugin = Plugin;
    g.PluginArray = PluginArray;
    g.MimeType = MimeType;
    g.MimeTypeArray = MimeTypeArray;
    g.MutationObserver = MutationObserver;
    g.ResizeObserver = ResizeObserver;
    g.IntersectionObserver = IntersectionObserver;
    g.PerformanceEntry = PerformanceEntry;
    g.PerformanceObserver = PerformanceObserver;
    g.AudioContext = audioContextFactory;
    g.OfflineAudioContext = OfflineAudioContext;
    g.webkitOfflineAudioContext = OfflineAudioContext;

    bindDocumentCookie(document, location);
    defineReadOnly(document, "defaultView", g);
})();
