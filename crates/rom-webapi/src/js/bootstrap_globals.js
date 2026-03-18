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
            applyLocationHref(this, nextHref);
        },
        replace(nextHref) {
            this.assign(nextHref);
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

    const navigator = {
        userAgent: navigatorConfig.userAgent ?? "ROM/0.1",
        appVersion: navigatorConfig.userAgent ?? "ROM/0.1",
        appName: navigatorConfig.appName ?? "Netscape",
        platform: navigatorConfig.platform ?? "unknown",
        language: navigatorConfig.language ?? "en-US",
        languages: navigatorConfig.languages ?? ["en-US"],
        hardwareConcurrency: navigatorConfig.hardwareConcurrency ?? 4,
        deviceMemory: navigatorConfig.deviceMemory ?? 8,
        cookieEnabled: true,
        webdriver: Boolean(navigatorConfig.webdriver),
        maxTouchPoints: 0,
        vendor: "ROM",
        product: "Gecko",
        productSub: "20030107",
        plugins: [],
        mimeTypes: [],
        userAgentData: null,
    };

    const history = {
        length: 1,
        state: null,
        back() {},
        forward() {},
        go() {},
        pushState(state, _title, nextHref) {
            this.state = state;
            if (nextHref) {
                location.assign(nextHref);
            }
        },
        replaceState(state, _title, nextHref) {
            this.state = state;
            if (nextHref) {
                location.replace(nextHref);
            }
        },
    };

    const screen = {
        width: 1920,
        height: 1080,
        availWidth: 1920,
        availHeight: 1040,
        availTop: 0,
        availLeft: 0,
        colorDepth: 24,
        pixelDepth: 24,
    };

    let nextTimerId = 1;
    const timers = new Map();

    function registerTimer(callback, interval, args) {
        const timerId = nextTimerId++;
        timers.set(timerId, { callback, interval, args });

        Promise.resolve().then(() => {
            const timer = timers.get(timerId);
            if (!timer || typeof callback !== "function") {
                return;
            }

            callback(...args);

            if (!interval) {
                timers.delete(timerId);
            }
        });

        return timerId;
    }

    function clearTimer(timerId) {
        timers.delete(timerId);
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

    const mediaQueryList = (query) => ({
        matches: false,
        media: String(query),
        onchange: null,
        addListener() {},
        removeListener() {},
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() {
            return true;
        },
    });

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

    const textEncoderFactory = class TextEncoder {
        encode(input = "") {
            return Uint8Array.from(String(input), (char) => char.charCodeAt(0));
        }
    };

    const textDecoderFactory = class TextDecoder {
        decode(input = new Uint8Array()) {
            return Array.from(input, (code) => String.fromCharCode(code)).join("");
        }
    };

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

    class Image {}

    class Audio {
        constructor() {
            this.currentTime = 0;
        }
    }

    class HTMLButtonElement extends Element {}

    class CompositionEvent extends Event {}

    g.window = g;
    g.self = g;
    g.top = g;
    g.parent = g;
    g.document = document;
    g.navigator = navigator;
    g.location = location;
    g.history = history;
    g.screen = screen;
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
    g.MessageChannel = MessageChannel;
    g.MessagePort = MessagePort;
    g.MessageEvent = MessageEvent;
    g.BroadcastChannel = BroadcastChannel;
    g.Worker = Worker;
    g.EventSource = EventSource;
    g.structuredClone = structuredClone;
    g.matchMedia = mediaQueryList;
    g.TextEncoder = textEncoderFactory;
    g.TextDecoder = textDecoderFactory;
    g.setTimeout = (callback, _delay, ...args) => registerTimer(callback, false, args);
    g.clearTimeout = clearTimer;
    g.setInterval = (callback, _delay, ...args) => registerTimer(callback, true, args);
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
    g.MutationObserver = class MutationObserver extends ObserverBase {};
    g.ResizeObserver = class ResizeObserver extends ObserverBase {};
    g.IntersectionObserver = class IntersectionObserver extends ObserverBase {};
    g.PerformanceObserver = class PerformanceObserver extends ObserverBase {};
    g.AudioContext = audioContextFactory;
    g.OfflineAudioContext = OfflineAudioContext;
    g.webkitOfflineAudioContext = OfflineAudioContext;

    bindDocumentCookie(document, location);
    defineReadOnly(document, "defaultView", g);
})();
