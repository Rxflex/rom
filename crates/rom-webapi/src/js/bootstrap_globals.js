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

    function dispatchHashChangeIfNeeded(previousHref, nextHref) {
        const previous = new URL(previousHref);
        const next = new URL(nextHref);
        if (
            previous.origin === next.origin &&
            previous.pathname === next.pathname &&
            previous.search === next.search &&
            previous.hash !== next.hash
        ) {
            dispatchWindowEvent(
                new HashChangeEvent("hashchange", {
                    oldURL: previous.href,
                    newURL: next.href,
                }),
            );
        }
    }

    function updateHistoryEntry(mode, state, nextHref, dispatchHashChange = false) {
        const previousHref = location.href;
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
        if (dispatchHashChange) {
            dispatchHashChangeIfNeeded(previousHref, entry.href);
        }
    }

    function navigateLocation(mode, nextHref) {
        const nextState = mode === "replace"
            ? historyEntries[currentHistoryIndex]?.state ?? null
            : null;
        updateHistoryEntry(mode, nextState, nextHref, true);
    }

    function traverseHistory(delta) {
        if (!delta) {
            return;
        }

        const nextIndex = currentHistoryIndex + delta;
        if (nextIndex < 0 || nextIndex >= historyEntries.length) {
            return;
        }

        const previousHref = location.href;
        currentHistoryIndex = nextIndex;
        const entry = historyEntries[currentHistoryIndex];
        syncHistoryState(entry);
        dispatchWindowEvent(
            new PopStateEvent("popstate", {
                state: cloneHistoryState(entry.state),
            }),
        );
        dispatchHashChangeIfNeeded(previousHref, entry.href);
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

            const hasEntryTypes = Array.isArray(options.entryTypes);
            const hasType = options.type !== undefined;
            const hasBuffered = options.buffered !== undefined;
            if (hasEntryTypes && (hasType || hasBuffered)) {
                throw new TypeError(
                    "PerformanceObserver cannot mix entryTypes with type or buffered.",
                );
            }

            let entryTypes = [];
            if (hasEntryTypes) {
                entryTypes = options.entryTypes.map(String);
            } else if (hasType) {
                entryTypes = [String(options.type)];
            }

            if (!entryTypes.length) {
                throw new TypeError("PerformanceObserver requires entryTypes or type.");
            }

            this.__entryTypes = entryTypes.filter(isSupportedPerformanceEntryType);
            if (hasType && options.buffered) {
                const bufferedEntries = getPerformanceEntries(this.__entryTypes[0]);
                enqueuePerformanceObserverRecords(this, bufferedEntries);
            }
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

            enqueuePerformanceObserverRecords(observer, [entry]);
        }
    }

    function enqueuePerformanceObserverRecords(observer, records) {
        if (!records.length) {
            return;
        }

        observer.__records.push(...records);
        if (observer.__scheduled) {
            return;
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

    function consoleMessage(args) {
        return args.map(String).join(" ");
    }

    function emitConsole(method, sinkName, args) {
        const sink = typeof g[sinkName] === "function"
            ? g[sinkName]
            : typeof g.__rom_console_log === "function"
                ? g.__rom_console_log
                : null;
        if (sink) {
            sink(consoleMessage(args));
        }
        return undefined;
    }

    const consoleCounters = new Map();
    const consoleTimers = new Map();

    const console = {
        log: (...args) => emitConsole("log", "__rom_console_log", args),
        info: (...args) => emitConsole("info", "__rom_console_log", args),
        debug: (...args) => emitConsole("debug", "__rom_console_log", args),
        warn: (...args) => emitConsole("warn", "__rom_console_warn", args),
        error: (...args) => emitConsole("error", "__rom_console_error", args),
        trace: (...args) => emitConsole("trace", "__rom_console_error", args),
        dir: (...args) => emitConsole("dir", "__rom_console_log", args),
        dirxml: (...args) => emitConsole("dirxml", "__rom_console_log", args),
        table: (...args) => emitConsole("table", "__rom_console_log", args),
        clear: () => undefined,
        group: (...args) => emitConsole("group", "__rom_console_log", args),
        groupCollapsed: (...args) => emitConsole("groupCollapsed", "__rom_console_log", args),
        groupEnd: () => undefined,
        count(label = "default") {
            const key = String(label);
            const nextValue = (consoleCounters.get(key) ?? 0) + 1;
            consoleCounters.set(key, nextValue);
            emitConsole("count", "__rom_console_log", [`${key}: ${nextValue}`]);
        },
        countReset(label = "default") {
            consoleCounters.set(String(label), 0);
        },
        time(label = "default") {
            consoleTimers.set(String(label), Date.now());
        },
        timeLog(label = "default", ...args) {
            const key = String(label);
            const startedAt = consoleTimers.get(key) ?? Date.now();
            emitConsole("timeLog", "__rom_console_log", [`${key}: ${Date.now() - startedAt}ms`, ...args]);
        },
        timeEnd(label = "default") {
            const key = String(label);
            const startedAt = consoleTimers.get(key) ?? Date.now();
            consoleTimers.delete(key);
            emitConsole("timeEnd", "__rom_console_log", [`${key}: ${Date.now() - startedAt}ms`]);
        },
        assert(condition, ...args) {
            if (!condition) {
                emitConsole(
                    "assert",
                    "__rom_console_error",
                    args.length ? args : ["Assertion failed"],
                );
            }
        },
        profile: () => undefined,
        profileEnd: () => undefined,
    };

    function supportsCssLength(value) {
        const normalized = String(value).trim().toLowerCase();
        return (
            normalized === "0" ||
            normalized === "auto" ||
            normalized === "normal" ||
            /^-?\d+(\.\d+)?(px|em|rem|vw|vh|vmin|vmax|%)$/.test(normalized) ||
            /^calc\(.+\)$/.test(normalized) ||
            /^var\(.+\)$/.test(normalized)
        );
    }

    function supportsCssColor(value) {
        const normalized = String(value).trim().toLowerCase();
        return (
            /^#[0-9a-f]{3,8}$/.test(normalized) ||
            /^(rgb|rgba|hsl|hsla|oklch|lab|lch)\(.+\)$/.test(normalized) ||
            /^var\(.+\)$/.test(normalized) ||
            /^(transparent|currentcolor|black|white|red|green|blue|gray|grey)$/.test(
                normalized,
            )
        );
    }

    function supportsCssTime(value) {
        const normalized = String(value).trim().toLowerCase();
        return /^-?\d+(\.\d+)?(ms|s)$/.test(normalized) || /^var\(.+\)$/.test(normalized);
    }

    function supportsCssTransform(value) {
        const normalized = String(value).trim();
        return (
            normalized === "none" ||
            /^var\(.+\)$/.test(normalized) ||
            /^[a-zA-Z-]+\(.+\)$/.test(normalized)
        );
    }

    function supportsCssPropertyValue(property, value) {
        const normalizedProperty = String(property).trim().toLowerCase();
        const normalizedValue = String(value).trim();
        if (!normalizedProperty || !normalizedValue) {
            return false;
        }

        if (normalizedProperty.startsWith("--")) {
            return true;
        }

        switch (normalizedProperty) {
            case "display":
                return /^(none|block|inline|inline-block|flex|grid|contents)$/.test(
                    normalizedValue.toLowerCase(),
                );
            case "position":
                return /^(static|relative|absolute|fixed|sticky)$/.test(
                    normalizedValue.toLowerCase(),
                );
            case "opacity": {
                const numeric = Number(normalizedValue);
                return Number.isFinite(numeric) && numeric >= 0 && numeric <= 1;
            }
            case "transform":
                return supportsCssTransform(normalizedValue);
            case "transition-duration":
            case "animation-duration":
                return supportsCssTime(normalizedValue);
            case "color":
            case "background":
            case "background-color":
            case "border-color":
            case "outline-color":
                return supportsCssColor(normalizedValue);
            case "width":
            case "height":
            case "min-width":
            case "min-height":
            case "max-width":
            case "max-height":
            case "margin":
            case "margin-top":
            case "margin-right":
            case "margin-bottom":
            case "margin-left":
            case "padding":
            case "padding-top":
            case "padding-right":
            case "padding-bottom":
            case "padding-left":
            case "top":
            case "right":
            case "bottom":
            case "left":
            case "font-size":
            case "border-radius":
                return supportsCssLength(normalizedValue);
            default:
                return false;
        }
    }

    function splitCssCondition(expression, operator) {
        const parts = [];
        let depth = 0;
        let start = 0;

        for (let index = 0; index < expression.length; index += 1) {
            const character = expression[index];
            if (character === "(") {
                depth += 1;
            } else if (character === ")") {
                depth = Math.max(0, depth - 1);
            }

            if (
                depth === 0 &&
                expression.slice(index, index + operator.length) === operator
            ) {
                parts.push(expression.slice(start, index).trim());
                start = index + operator.length;
                index += operator.length - 1;
            }
        }

        if (parts.length === 0) {
            return null;
        }

        parts.push(expression.slice(start).trim());
        return parts;
    }

    function unwrapCssCondition(expression) {
        let normalized = expression.trim();
        while (
            normalized.startsWith("(") &&
            normalized.endsWith(")") &&
            hasBalancedCssParentheses(normalized.slice(1, -1))
        ) {
            normalized = normalized.slice(1, -1).trim();
        }
        return normalized;
    }

    function hasBalancedCssParentheses(expression) {
        let depth = 0;
        for (const character of expression) {
            if (character === "(") {
                depth += 1;
            } else if (character === ")") {
                depth -= 1;
                if (depth < 0) {
                    return false;
                }
            }
        }
        return depth === 0;
    }

    function evaluateCssSupportsCondition(conditionText) {
        const condition = unwrapCssCondition(String(conditionText));
        if (!condition) {
            return false;
        }

        if (condition.startsWith("not ")) {
            return !evaluateCssSupportsCondition(condition.slice(4));
        }

        const orParts = splitCssCondition(condition, " or ");
        if (orParts) {
            return orParts.some((part) => evaluateCssSupportsCondition(part));
        }

        const andParts = splitCssCondition(condition, " and ");
        if (andParts) {
            return andParts.every((part) => evaluateCssSupportsCondition(part));
        }

        const declaration = unwrapCssCondition(condition);
        const separatorIndex = declaration.indexOf(":");
        if (separatorIndex < 0) {
            return false;
        }

        const property = declaration.slice(0, separatorIndex).trim();
        const value = declaration.slice(separatorIndex + 1).trim();
        return supportsCssPropertyValue(property, value);
    }

    const CSS = {
        supports(propertyOrConditionText, value = undefined) {
            if (value !== undefined) {
                return supportsCssPropertyValue(propertyOrConditionText, value);
            }

            return evaluateCssSupportsCondition(propertyOrConditionText);
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

    class HashChangeEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.oldURL = String(init.oldURL ?? "");
            this.newURL = String(init.newURL ?? "");
        }
    }

    class PromiseRejectionEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.promise = init.promise ?? null;
            this.reason = init.reason ?? null;
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

    function invokeWindowListener(listener, event) {
        if (typeof listener === "function") {
            listener.call(g, event);
            return;
        }

        if (listener && typeof listener.handleEvent === "function") {
            listener.handleEvent.call(listener, event);
        }
    }

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
                invokeWindowListener(entry.listener, instance);
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

    function markCompletedLifecycleEvent(target, type) {
        if (!target.__romCompletedEvents) {
            target.__romCompletedEvents = new Set();
        }
        target.__romCompletedEvents.add(String(type));
    }

    function dispatchCompletedLifecycleEvent(target, event) {
        markCompletedLifecycleEvent(target, event.type);
        if (target === g) {
            return dispatchWindowEvent(event);
        }
        return target.dispatchEvent(event);
    }

    function scheduleStartupLifecycleEvents() {
        Promise.resolve().then(() => {
            dispatchCompletedLifecycleEvent(document, new Event("readystatechange"));
            dispatchCompletedLifecycleEvent(document, new Event("DOMContentLoaded"));
            dispatchCompletedLifecycleEvent(g, new Event("load"));
            const pageShowEvent = new Event("pageshow");
            pageShowEvent.persisted = false;
            dispatchCompletedLifecycleEvent(g, pageShowEvent);
        });
    }

    function webpackChunkGlobalKeys() {
        return Object.getOwnPropertyNames(g).filter((key) => (
            key === "__LOADABLE_LOADED_CHUNKS__" ||
            /^__LOADABLE_LOADED_CHUNKS__/.test(key) ||
            /^webpackChunk/.test(key)
        ));
    }

    function probeWebpackChunkGlobal(chunkGlobal) {
        if (!Array.isArray(chunkGlobal) || typeof chunkGlobal.push !== "function") {
            return null;
        }

        let capturedRequire = null;
        const probeChunkId = `__rom_webpack_probe__${Date.now()}_${Math.random().toString(36).slice(2)}`;
        try {
            chunkGlobal.push([
                [probeChunkId],
                {},
                (webpackRequire) => {
                    capturedRequire = webpackRequire;
                    return undefined;
                },
            ]);
        } catch (_error) {
            return null;
        }

        return typeof capturedRequire === "function" ? capturedRequire : null;
    }

    async function loadWebpackScript(url) {
        if (!g.__romWebpackScriptLoads) {
            g.__romWebpackScriptLoads = new Map();
        }

        const normalizedUrl = String(url ?? "");
        if (g.__romWebpackScriptLoads.has(normalizedUrl)) {
            return g.__romWebpackScriptLoads.get(normalizedUrl);
        }

        const task = (async () => {
            const response = await g.fetch(normalizedUrl);
            if (!response || response.status >= 400) {
                throw new Error(`Failed to load script: ${normalizedUrl}`);
            }

            const source = await response.text();
            (0, g.eval)(source);
            return undefined;
        })();

        const trackedTask = task.finally(() => {
            g.__romWebpackScriptLoads.delete(normalizedUrl);
        });
        g.__romWebpackScriptLoads.set(normalizedUrl, trackedTask);
        return trackedTask;
    }

    function installWebpackRuntimePatches(webpackRequire) {
        if (typeof webpackRequire !== "function" || webpackRequire.__romPatched) {
            return webpackRequire;
        }

        Object.defineProperty(webpackRequire, "__romPatched", {
            configurable: true,
            enumerable: false,
            writable: true,
            value: true,
        });

        if (typeof webpackRequire.l === "function") {
            webpackRequire.l = (url, done) => {
                loadWebpackScript(url)
                    .then(() => {
                        exposeWebpackRequireFromChunkGlobals();
                        if (typeof done === "function") {
                            done({ type: "load", target: { src: url } });
                        }
                    })
                    .catch((error) => {
                        if (typeof done === "function") {
                            done({ type: "error", target: { src: url }, error });
                        }
                    });
            };
        }

        if (webpackRequire.f && typeof webpackRequire.f.miniCss === "function") {
            const loadedCssChunks = new Set();
            webpackRequire.f.miniCss = (chunkId, promises) => {
                if (loadedCssChunks.has(chunkId)) {
                    return;
                }
                loadedCssChunks.add(chunkId);
                promises.push(Promise.resolve());
            };
        }

        if (webpackRequire.F && typeof webpackRequire.F.j === "function") {
            webpackRequire.F.j = () => undefined;
        }

        return webpackRequire;
    }

    function exposeWebpackRequireFromChunkGlobals() {
        if (typeof g.__webpack_require__ === "function") {
            return installWebpackRuntimePatches(g.__webpack_require__);
        }

        for (const key of webpackChunkGlobalKeys()) {
            const webpackRequire = probeWebpackChunkGlobal(g[key]);
            if (typeof webpackRequire === "function") {
                g.__webpack_require__ = installWebpackRuntimePatches(webpackRequire);
                return g.__webpack_require__;
            }
        }

        return null;
    }

    function loadExternalScriptViaDom(url, integrity = "") {
        return new Promise((resolve, reject) => {
            const script = document.createElement("script");
            script.src = String(url);
            script.async = false;
            script.crossOrigin = "anonymous";
            if (integrity) {
                script.integrity = String(integrity);
            }
            script.onload = () => resolve(script.src);
            script.onerror = (event) =>
                reject(event?.error ?? new Error(`Failed to load script: ${url}`));
            document.head.appendChild(script);
        });
    }

    function installXRenderResourcesLoaderPatches(loader) {
        if (!loader || typeof loader !== "object" || loader.__romPatchedXRenderLoader) {
            return loader;
        }

        Object.defineProperty(loader, "__romPatchedXRenderLoader", {
            configurable: true,
            enumerable: false,
            writable: false,
            value: true,
        });

        loader.loadScript = function loadScript(url, integrity = "", _immediate = false) {
            return loadExternalScriptViaDom(url, integrity);
        };

        loader.loadScripts = function loadScripts(urls, integrities = [], immediate = false) {
            return Array.from(urls ?? [], (url, index) =>
                loader.loadScript(url, integrities?.[index] ?? "", immediate));
        };

        return loader;
    }

    function interceptXRenderResourcesLoader() {
        let currentValue = installXRenderResourcesLoaderPatches(g.__XRenderResourcesLoader ?? null);

        try {
            delete g.__XRenderResourcesLoader;
        } catch (_error) {
            return;
        }

        Object.defineProperty(g, "__XRenderResourcesLoader", {
            configurable: true,
            enumerable: true,
            get() {
                return currentValue;
            },
            set(value) {
                currentValue = installXRenderResourcesLoaderPatches(value);
            },
        });
    }

    const nativeEval = g.eval;

    function normalizeEvalScriptSource(source) {
        return String(source ?? "")
            .replace(/\r\n?/g, "\n")
            .trim();
    }

    function findExistingEvalScript(source) {
        if (!document || typeof document.scripts?.find !== "function") {
            return null;
        }

        const normalized = normalizeEvalScriptSource(source);
        if (!normalized) {
            return null;
        }

        return document.scripts.find((script) => {
            if (script.getAttribute?.("src")) {
                return false;
            }

            const scriptSource = normalizeEvalScriptSource(script.textContent ?? "");
            return (
                scriptSource === normalized ||
                scriptSource.includes(normalized) ||
                normalized.includes(scriptSource)
            );
        }) ?? null;
    }

    function pushEvalScriptContext(source) {
        if (!document) {
            return null;
        }

        const currentScript = document.currentScript;
        const existingScript = findExistingEvalScript(source);
        if (currentScript && !currentScript.__romSyntheticEvalScript && !existingScript) {
            return null;
        }

        if (existingScript) {
            if (!document.__currentScriptStack) {
                document.__currentScriptStack = [];
            }
            document.__currentScriptStack.push(existingScript);
            document.__currentScript = existingScript;
            return existingScript;
        }

        const syntheticScript = document.createElement("script");
        syntheticScript.text = "";
        syntheticScript.__romSyntheticEvalScript = true;
        if (!document.__currentScriptStack) {
            document.__currentScriptStack = [];
        }
        document.__currentScriptStack.push(syntheticScript);
        document.__currentScript = syntheticScript;
        return syntheticScript;
    }

    function popEvalScriptContext(scriptNode) {
        if (!scriptNode || !document) {
            return;
        }

        const stack = document.__currentScriptStack ?? [];
        if (stack[stack.length - 1] === scriptNode) {
            stack.pop();
        } else {
            const index = stack.lastIndexOf(scriptNode);
            if (index >= 0) {
                stack.splice(index, 1);
            }
        }
        document.__currentScript = stack[stack.length - 1] ?? null;
    }

    function isInertInlineScriptPayload(source) {
        const normalized = String(source ?? "").trim();
        if (!normalized || !/^[{\[]/.test(normalized)) {
            return false;
        }

        try {
            JSON.parse(normalized);
            return true;
        } catch (_error) {
            return false;
        }
    }

    function flushPendingSyntheticDomContentLoaded() {
        if (!document?.__romPendingSyntheticDomContentLoaded) {
            return;
        }

        document.__romPendingSyntheticDomContentLoaded = false;
        document.dispatchEvent(new Event("DOMContentLoaded"));
        document.__romSuppressDomContentLoadedReplay = false;
    }

    function evalWithWebpackExposure(source) {
        const scriptNode = pushEvalScriptContext(source);
        try {
            if (isInertInlineScriptPayload(source)) {
                flushPendingSyntheticDomContentLoaded();
                exposeWebpackRequireFromChunkGlobals();
                return undefined;
            }
            const result = nativeEval(source);
            flushPendingSyntheticDomContentLoaded();
            exposeWebpackRequireFromChunkGlobals();
            return result;
        } finally {
            popEvalScriptContext(scriptNode);
        }
    }

    g.window = g;
    g.self = g;
    g.top = g;
    g.parent = g;
    g.__listeners = new Map();
    g.__romCompletedEvents = new Set();
    document.__romCompletedEvents = new Set();
    g.__REGION_CONFIG__ = {};
    g.addEventListener = EventTarget.prototype.addEventListener.bind(g);
    g.removeEventListener = EventTarget.prototype.removeEventListener.bind(g);
    g.dispatchEvent = (event) => dispatchWindowEvent(event);
    g.__rom_expose_webpack_require = () => exposeWebpackRequireFromChunkGlobals();
    interceptXRenderResourcesLoader();
    g.eval = evalWithWebpackExposure;
    g.onpopstate = null;
    g.onhashchange = null;
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
    g.localStorage.importState(initialLocalStorage);
    g.sessionStorage.importState(initialSessionStorage);
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
    g.HashChangeEvent = HashChangeEvent;
    g.PromiseRejectionEvent = PromiseRejectionEvent;
    g.onunhandledrejection = null;
    g.onrejectionhandled = null;
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
    g.__rom_export_local_storage = () => g.localStorage.exportState();
    g.__rom_export_session_storage = () => g.sessionStorage.exportState();
    g.queueMicrotask = (callback) =>
        Promise.resolve().then(() => {
            if (typeof callback === "function") {
                callback();
            }
        });
    g.requestAnimationFrame = (callback) =>
        registerTimer(() => callback(performance.now()), false, 16, []);
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
    g.HTMLAnchorElement = HTMLAnchorElement;
    g.HTMLCanvasElement = HTMLCanvasElement;
    g.HTMLIFrameElement = HTMLIFrameElement;
    g.HTMLLinkElement = HTMLLinkElement;
    g.HTMLScriptElement = HTMLScriptElement;
    g.Text = Text;
    g.Comment = Comment;
    g.Document = Document;
    g.DocumentFragment = DocumentFragment;
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
    g.TextMetrics = TextMetrics;
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
    defineReadOnly(document, "referrer", String(documentConfig.referrer ?? ""));
    defineReadOnly(document, "defaultView", g);
    scheduleStartupLifecycleEvents();
})();
