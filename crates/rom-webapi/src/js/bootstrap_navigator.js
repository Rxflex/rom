    class PermissionStatus extends EventTarget {
        constructor(name, state) {
            super();
            this.name = String(name);
            this.state = String(state);
            this.onchange = null;
        }
    }

    class Permissions {
        constructor(permissionState) {
            this.__permissionState = permissionState;
        }

        query(descriptor = {}) {
            const name = String(descriptor.name ?? "");
            return Promise.resolve(
                new PermissionStatus(name, this.__permissionState.get(name) ?? "prompt"),
            );
        }
    }

    class NavigatorUAData {
        constructor(init) {
            this.brands = Array.from(init.brands ?? [], cloneBrandEntry);
            this.mobile = Boolean(init.mobile);
            this.platform = String(init.platform ?? "Unknown");
            this.__highEntropyValues = {
                architecture: String(init.architecture ?? "x86"),
                bitness: String(init.bitness ?? "64"),
                formFactors: Array.from(init.formFactors ?? []),
                model: String(init.model ?? ""),
                platformVersion: String(init.platformVersion ?? "0.0.0"),
                uaFullVersion: String(init.uaFullVersion ?? "0.1.0"),
                fullVersionList: Array.from(init.fullVersionList ?? this.brands, cloneBrandEntry),
                wow64: Boolean(init.wow64),
            };
        }

        toJSON() {
            return {
                brands: this.brands.map(cloneBrandEntry),
                mobile: this.mobile,
                platform: this.platform,
            };
        }

        getHighEntropyValues(hints = []) {
            const result = this.toJSON();
            for (const hint of Array.from(hints, String)) {
                if (this.__highEntropyValues[hint] !== undefined) {
                    result[hint] = cloneHighEntropyValue(this.__highEntropyValues[hint]);
                }
            }
            return Promise.resolve(result);
        }
    }

    class MediaTrackSettings {
        constructor(kind) {
            this.deviceId = `${kind}-default`;
            this.groupId = `${kind}-group`;
        }
    }

    class MediaStreamTrack extends EventTarget {
        constructor(kind, label) {
            super();
            this.kind = kind;
            this.label = label;
            this.enabled = true;
            this.id = `${kind}-${Math.random().toString(16).slice(2, 10)}`;
            this.muted = false;
            this.readyState = "live";
            this.onended = null;
            this.__settings = new MediaTrackSettings(kind);
        }

        stop() {
            if (this.readyState === "ended") {
                return;
            }

            this.readyState = "ended";
            queueMicrotask(() => {
                const event = new Event("ended");
                if (typeof this.onended === "function") {
                    this.onended(event);
                }
                this.dispatchEvent(event);
            });
        }

        clone() {
            return new MediaStreamTrack(this.kind, this.label);
        }

        getCapabilities() {
            return {};
        }

        getConstraints() {
            return {};
        }

        getSettings() {
            return { ...this.__settings };
        }

        applyConstraints() {
            return Promise.resolve();
        }
    }

    class MediaStream extends EventTarget {
        constructor(tracks = []) {
            super();
            this.id = `stream-${Math.random().toString(16).slice(2, 10)}`;
            this.active = true;
            this.onaddtrack = null;
            this.onremovetrack = null;
            this.__tracks = Array.from(tracks);
        }

        getTracks() {
            return this.__tracks.slice();
        }

        getAudioTracks() {
            return this.__tracks.filter((track) => track.kind === "audio");
        }

        getVideoTracks() {
            return this.__tracks.filter((track) => track.kind === "video");
        }

        getTrackById(trackId) {
            return this.__tracks.find((track) => track.id === String(trackId)) ?? null;
        }

        addTrack(track) {
            this.__tracks.push(track);
        }

        removeTrack(track) {
            this.__tracks = this.__tracks.filter((entry) => entry !== track);
        }

        clone() {
            return new MediaStream(this.__tracks.map((track) => track.clone()));
        }
    }

    class MediaDeviceInfo {
        constructor(kind, label, deviceId, groupId) {
            this.kind = kind;
            this.label = label;
            this.deviceId = deviceId;
            this.groupId = groupId;
        }

        toJSON() {
            return {
                kind: this.kind,
                label: this.label,
                deviceId: this.deviceId,
                groupId: this.groupId,
            };
        }
    }

    class InputDeviceInfo extends MediaDeviceInfo {}

    class MediaDevices extends EventTarget {
        constructor(devices, permissionState) {
            super();
            this.ondevicechange = null;
            this.__devices = devices;
            this.__permissionState = permissionState;
        }

        enumerateDevices() {
            return Promise.resolve(
                this.__devices.map((device) =>
                    cloneMediaDevice(device, shouldRevealDeviceLabel(device, this.__permissionState)),
                ),
            );
        }

        getSupportedConstraints() {
            return {
                audio: true,
                video: true,
                deviceId: true,
                facingMode: true,
                frameRate: true,
                height: true,
                width: true,
            };
        }

        getUserMedia(constraints = {}) {
            const normalized = normalizeMediaConstraints(constraints);
            const tracks = [];

            if (normalized.audio) {
                this.__permissionState.set("microphone", "granted");
                tracks.push(new MediaStreamTrack("audio", "Default Audio Input"));
            }
            if (normalized.video) {
                this.__permissionState.set("camera", "granted");
                tracks.push(new MediaStreamTrack("video", "Default Video Input"));
            }
            if (!tracks.length) {
                return Promise.reject(new TypeError("At least one media constraint must be requested."));
            }

            this.__queueDeviceChange();
            return Promise.resolve(new MediaStream(tracks));
        }

        getDisplayMedia(constraints = {}) {
            const normalized = normalizeMediaConstraints(constraints);
            if (!normalized.video) {
                return Promise.reject(new TypeError("Display capture requires video."));
            }

            this.__queueDeviceChange();
            return Promise.resolve(
                new MediaStream([new MediaStreamTrack("video", "ROM Display Capture")]),
            );
        }

        __queueDeviceChange() {
            queueMicrotask(() => {
                const event = new Event("devicechange");
                if (typeof this.ondevicechange === "function") {
                    this.ondevicechange(event);
                }
                this.dispatchEvent(event);
            });
        }
    }

    class Plugin {
        constructor(name, filename, description) {
            this.name = name;
            this.filename = filename;
            this.description = description;
            this.length = 0;
        }

        item(index) {
            return this[index] ?? null;
        }

        namedItem(name) {
            return this[String(name)] ?? null;
        }
    }

    class MimeType {
        constructor(type, suffixes, description, enabledPlugin = null) {
            this.type = type;
            this.suffixes = suffixes;
            this.description = description;
            this.enabledPlugin = enabledPlugin;
        }
    }

    class PluginArray {
        constructor(entries) {
            this.length = 0;
            for (const [index, entry] of entries.entries()) {
                this[index] = entry;
                this[entry.name] = entry;
                this.length += 1;
            }
        }

        item(index) {
            return this[index] ?? null;
        }

        namedItem(name) {
            return this[String(name)] ?? null;
        }

        refresh() {}

        [Symbol.iterator]() {
            return Array.from({ length: this.length }, (_, index) => this[index])[Symbol.iterator]();
        }
    }

    class MimeTypeArray {
        constructor(entries) {
            this.length = 0;
            for (const [index, entry] of entries.entries()) {
                this[index] = entry;
                this[entry.type] = entry;
                this.length += 1;
            }
        }

        item(index) {
            return this[index] ?? null;
        }

        namedItem(name) {
            return this[String(name)] ?? null;
        }

        [Symbol.iterator]() {
            return Array.from({ length: this.length }, (_, index) => this[index])[Symbol.iterator]();
        }
    }

    function createNavigator(navigatorConfig) {
        const permissionState = createPermissionState();
        const mimeTypes = createDefaultMimeTypes();
        const plugins = createDefaultPlugins(mimeTypes);
        const mediaDevices = createDefaultMediaDevices(permissionState);
        const userAgentData = createNavigatorUAData(navigatorConfig);

        return {
            userAgent:
                navigatorConfig.userAgent ??
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            appVersion:
                navigatorConfig.userAgent ??
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36",
            appName: navigatorConfig.appName ?? "Netscape",
            platform: navigatorConfig.platform ?? "unknown",
            language: navigatorConfig.language ?? "en-US",
            languages: navigatorConfig.languages ?? ["en-US"],
            hardwareConcurrency: navigatorConfig.hardwareConcurrency ?? 4,
            deviceMemory: navigatorConfig.deviceMemory ?? 8,
            cookieEnabled: true,
            webdriver: Boolean(navigatorConfig.webdriver),
            maxTouchPoints: 0,
            vendor: "Google Inc.",
            product: "Gecko",
            productSub: "20030107",
            userAgentData,
            plugins,
            mimeTypes,
            permissions: new Permissions(permissionState),
            mediaDevices,
            pdfViewerEnabled: true,
        };
    }

    function createDefaultPlugins(mimeTypes) {
        const pdfPlugin = new Plugin("PDF Viewer", "internal-pdf-viewer", "Portable Document Format");
        pdfPlugin[0] = mimeTypes[0];
        pdfPlugin.length = 1;
        mimeTypes[0].enabledPlugin = pdfPlugin;
        return new PluginArray([pdfPlugin]);
    }

    function createDefaultMimeTypes() {
        return new MimeTypeArray([
            new MimeType("application/pdf", "pdf", "Portable Document Format"),
        ]);
    }

    function createDefaultMediaDevices(permissionState) {
        return new MediaDevices([
            new InputDeviceInfo("audioinput", "Default Audio Input", "audioinput-default", "media-group"),
            new InputDeviceInfo("videoinput", "Default Video Input", "videoinput-default", "media-group"),
            new MediaDeviceInfo("audiooutput", "Default Audio Output", "audiooutput-default", "media-group"),
        ], permissionState);
    }

    function createNavigatorUAData(navigatorConfig) {
        const platform = normalizeUaPlatform(navigatorConfig.platform);
        const brands = [
            { brand: "Chromium", version: "137" },
            { brand: "Google Chrome", version: "137" },
            { brand: "Not=A?Brand", version: "24" },
        ];

        return new NavigatorUAData({
            brands,
            mobile: false,
            platform,
            architecture: "x86",
            bitness: "64",
            formFactors: ["Desktop"],
            model: "",
            platformVersion: "15.0.0",
            uaFullVersion: "137.0.0.0",
            fullVersionList: brands,
            wow64: false,
        });
    }

    function normalizeMediaConstraints(constraints) {
        return {
            audio: normalizeMediaConstraintValue(constraints.audio),
            video: normalizeMediaConstraintValue(constraints.video),
        };
    }

    function normalizeMediaConstraintValue(value) {
        if (value === undefined || value === null) {
            return false;
        }
        if (typeof value === "boolean") {
            return value;
        }
        if (typeof value === "object") {
            return true;
        }
        return Boolean(value);
    }

    function cloneMediaDevice(device, revealLabel) {
        const label = revealLabel ? device.label : "";
        if (device instanceof InputDeviceInfo) {
            return new InputDeviceInfo(device.kind, label, device.deviceId, device.groupId);
        }
        return new MediaDeviceInfo(device.kind, label, device.deviceId, device.groupId);
    }

    function cloneBrandEntry(entry) {
        return {
            brand: String(entry?.brand ?? ""),
            version: String(entry?.version ?? ""),
        };
    }

    function cloneHighEntropyValue(value) {
        if (Array.isArray(value)) {
            return value.slice();
        }
        if (value && typeof value === "object") {
            return { ...value };
        }
        return value;
    }

    function normalizeUaPlatform(platform) {
        const value = String(platform ?? "").toLowerCase();
        if (value.includes("win")) {
            return "Windows";
        }
        if (value.includes("mac")) {
            return "macOS";
        }
        if (value.includes("linux")) {
            return "Linux";
        }
        return "Unknown";
    }

    function createPermissionState() {
        return new Map([
            ["camera", "prompt"],
            ["microphone", "prompt"],
            ["notifications", "default"],
        ]);
    }

    function shouldRevealDeviceLabel(device, permissionState) {
        if (device.kind === "audioinput" || device.kind === "audiooutput") {
            return permissionState.get("microphone") === "granted";
        }

        if (device.kind === "videoinput") {
            return permissionState.get("camera") === "granted";
        }

        return false;
    }
