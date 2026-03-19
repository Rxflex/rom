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

    const performance = {
        timeOrigin: nowBase,
        now() {
            return Date.now() - nowBase;
        },
        mark() {},
        measure() {},
        getEntries() {
            return [];
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
    g.PerformanceObserver = class PerformanceObserver extends ObserverBase {};
    g.AudioContext = audioContextFactory;
    g.OfflineAudioContext = OfflineAudioContext;
    g.webkitOfflineAudioContext = OfflineAudioContext;

    bindDocumentCookie(document, location);
    defineReadOnly(document, "defaultView", g);
})();
