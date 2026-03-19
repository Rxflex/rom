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

    const navigator = createNavigator(navigatorConfig);

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

    const textEncoderFactory = class TextEncoder {
        constructor() {
            defineReadOnly(this, "encoding", "utf-8");
        }

        encode(input = "") {
            return Uint8Array.from(encodeUtf8(String(input)));
        }
    };

    const textDecoderFactory = class TextDecoder {
        constructor(label = "utf-8", options = {}) {
            const normalizedLabel = String(label).toLowerCase();
            if (normalizedLabel !== "utf-8" && normalizedLabel !== "utf8") {
                throw new RangeError(`Unsupported encoding: ${label}`);
            }

            defineReadOnly(this, "encoding", "utf-8");
            defineReadOnly(this, "fatal", Boolean(options.fatal));
            defineReadOnly(this, "ignoreBOM", Boolean(options.ignoreBOM));
        }

        decode(input = new Uint8Array()) {
            return decodeUtf8(normalizeTextDecodeInput(input), {
                fatal: this.fatal,
                ignoreBOM: this.ignoreBOM,
            });
        }
    };

    function encodeUtf8(input) {
        const bytes = [];

        for (const symbol of input) {
            const codePoint = symbol.codePointAt(0);

            if (codePoint <= 0x7f) {
                bytes.push(codePoint);
                continue;
            }

            if (codePoint <= 0x7ff) {
                bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
                continue;
            }

            if (codePoint <= 0xffff) {
                bytes.push(
                    0xe0 | (codePoint >> 12),
                    0x80 | ((codePoint >> 6) & 0x3f),
                    0x80 | (codePoint & 0x3f),
                );
                continue;
            }

            bytes.push(
                0xf0 | (codePoint >> 18),
                0x80 | ((codePoint >> 12) & 0x3f),
                0x80 | ((codePoint >> 6) & 0x3f),
                0x80 | (codePoint & 0x3f),
            );
        }

        return bytes;
    }

    function normalizeTextDecodeInput(input) {
        if (input instanceof Uint8Array) {
            return input;
        }

        if (ArrayBuffer.isView(input)) {
            return new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
        }

        if (input instanceof ArrayBuffer) {
            return new Uint8Array(input);
        }

        return Uint8Array.from(input ?? []);
    }

    function decodeUtf8(input, options = {}) {
        const bytes = Array.from(input);
        const codeUnits = [];
        const fatal = Boolean(options.fatal);
        let index = 0;

        if (
            !options.ignoreBOM &&
            bytes.length >= 3 &&
            bytes[0] === 0xef &&
            bytes[1] === 0xbb &&
            bytes[2] === 0xbf
        ) {
            index = 3;
        }

        while (index < bytes.length) {
            const first = bytes[index];

            if (first <= 0x7f) {
                codeUnits.push(first);
                index += 1;
                continue;
            }

            const sequence = readUtf8Sequence(bytes, index);
            if (sequence === null) {
                if (fatal) {
                    throw new TypeError("The encoded data was not valid utf-8.");
                }

                codeUnits.push(0xfffd);
                index += 1;
                continue;
            }

            const { codePoint, length } = sequence;
            if (codePoint <= 0xffff) {
                codeUnits.push(codePoint);
            } else {
                const adjusted = codePoint - 0x10000;
                codeUnits.push(0xd800 + (adjusted >> 10), 0xdc00 + (adjusted & 0x3ff));
            }

            index += length;
        }

        return String.fromCharCode(...codeUnits);
    }

    function readUtf8Sequence(bytes, index) {
        const first = bytes[index];

        if (first >= 0xc2 && first <= 0xdf) {
            const second = bytes[index + 1];
            if (!isUtf8ContinuationByte(second)) {
                return null;
            }

            return {
                codePoint: ((first & 0x1f) << 6) | (second & 0x3f),
                length: 2,
            };
        }

        if (first >= 0xe0 && first <= 0xef) {
            const second = bytes[index + 1];
            const third = bytes[index + 2];
            if (
                !isUtf8ContinuationByte(second) ||
                !isUtf8ContinuationByte(third) ||
                (first === 0xe0 && second < 0xa0) ||
                (first === 0xed && second >= 0xa0)
            ) {
                return null;
            }

            return {
                codePoint:
                    ((first & 0x0f) << 12) | ((second & 0x3f) << 6) | (third & 0x3f),
                length: 3,
            };
        }

        if (first >= 0xf0 && first <= 0xf4) {
            const second = bytes[index + 1];
            const third = bytes[index + 2];
            const fourth = bytes[index + 3];
            if (
                !isUtf8ContinuationByte(second) ||
                !isUtf8ContinuationByte(third) ||
                !isUtf8ContinuationByte(fourth) ||
                (first === 0xf0 && second < 0x90) ||
                (first === 0xf4 && second >= 0x90)
            ) {
                return null;
            }

            return {
                codePoint:
                    ((first & 0x07) << 18) |
                    ((second & 0x3f) << 12) |
                    ((third & 0x3f) << 6) |
                    (fourth & 0x3f),
                length: 4,
            };
        }

        return null;
    }

    function isUtf8ContinuationByte(value) {
        return Number.isInteger(value) && (value & 0xc0) === 0x80;
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
