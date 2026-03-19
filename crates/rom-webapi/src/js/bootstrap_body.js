    class Headers {
        constructor(init = undefined) {
            this.__entries = [];

            if (!init) {
                return;
            }

            if (init instanceof Headers) {
                this.__entries = init.entries();
                return;
            }

            if (Array.isArray(init)) {
                for (const entry of init) {
                    if (Array.isArray(entry) && entry.length >= 2) {
                        this.append(entry[0], entry[1]);
                    }
                }
                return;
            }

            for (const key of Object.keys(init)) {
                this.append(key, init[key]);
            }
        }

        append(name, value) {
            this.__entries.push([String(name).toLowerCase(), String(value)]);
        }

        set(name, value) {
            const normalized = String(name).toLowerCase();
            this.delete(normalized);
            this.append(normalized, value);
        }

        get(name) {
            const normalized = String(name).toLowerCase();
            const values = this.__entries
                .filter(([key]) => key === normalized)
                .map(([, value]) => value);
            return values.length > 0 ? values.join(", ") : null;
        }

        has(name) {
            const normalized = String(name).toLowerCase();
            return this.__entries.some(([key]) => key === normalized);
        }

        delete(name) {
            const normalized = String(name).toLowerCase();
            this.__entries = this.__entries.filter(([key]) => key !== normalized);
        }

        entries() {
            return this.__entries.map(([name, value]) => [name, value]);
        }

        keys() {
            return this.__entries.map(([name]) => name);
        }

        values() {
            return this.__entries.map(([, value]) => value);
        }

        forEach(callback, thisArg = undefined) {
            for (const [name, value] of this.__entries) {
                callback.call(thisArg, value, name, this);
            }
        }

        [Symbol.iterator]() {
            return this.entries()[Symbol.iterator]();
        }
    }

    class Blob {
        constructor(parts = [], options = {}) {
            this.type = String(options.type ?? "").toLowerCase();
            this.__bytes = flattenParts(parts);
            this.size = this.__bytes.length;
        }

        async text() {
            return decodeBytes(this.__bytes);
        }

        async arrayBuffer() {
            return Uint8Array.from(this.__bytes).buffer;
        }

        slice(start = 0, end = this.size, type = "") {
            return new Blob([this.__bytes.slice(start, end)], { type });
        }
    }

    class File extends Blob {
        constructor(parts, name, options = {}) {
            super(parts, options);
            this.name = String(name);
            this.lastModified = Number(options.lastModified ?? Date.now());
        }
    }

    class FormData {
        constructor() {
            this.__entries = [];
        }

        append(name, value, filename = undefined) {
            this.__entries.push({
                name: String(name),
                value,
                filename: filename === undefined ? undefined : String(filename),
            });
        }

        set(name, value, filename = undefined) {
            this.delete(name);
            this.append(name, value, filename);
        }

        get(name) {
            const entry = this.__entries.find((item) => item.name === String(name));
            return entry ? entry.value : null;
        }

        getAll(name) {
            return this.__entries
                .filter((item) => item.name === String(name))
                .map((item) => item.value);
        }

        has(name) {
            return this.__entries.some((item) => item.name === String(name));
        }

        delete(name) {
            this.__entries = this.__entries.filter((item) => item.name !== String(name));
        }

        entries() {
            return this.__entries.map((item) => [item.name, item.value]);
        }

        keys() {
            return this.__entries.map((item) => item.name);
        }

        values() {
            return this.__entries.map((item) => item.value);
        }

        forEach(callback, thisArg = undefined) {
            for (const entry of this.__entries) {
                callback.call(thisArg, entry.value, entry.name, this);
            }
        }

        [Symbol.iterator]() {
            return this.entries()[Symbol.iterator]();
        }
    }

    class AbortSignal extends EventTarget {
        constructor() {
            super();
            this.aborted = false;
            this.reason = undefined;
        }

        throwIfAborted() {
            if (this.aborted) {
                throw this.reason ?? new Error("The operation was aborted.");
            }
        }
    }

    class AbortController {
        constructor() {
            this.signal = new AbortSignal();
        }

        abort(reason = new Error("The operation was aborted.")) {
            if (this.signal.aborted) {
                return;
            }

            this.signal.aborted = true;
            this.signal.reason = reason;
            this.signal.dispatchEvent(new Event("abort"));
        }
    }

    class FileReader extends EventTarget {
        constructor() {
            super();
            this.readyState = FileReader.EMPTY;
            this.result = null;
            this.error = null;
            this.onloadstart = null;
            this.onprogress = null;
            this.onload = null;
            this.onloadend = null;
            this.onabort = null;
            this.onerror = null;
            this.__aborted = false;
        }

        abort() {
            if (this.readyState !== FileReader.LOADING) {
                this.result = null;
                return;
            }

            this.__aborted = true;
            this.readyState = FileReader.DONE;
            this.result = null;
            dispatchFileReaderEvent(this, "abort");
            dispatchFileReaderEvent(this, "loadend");
        }

        readAsText(blob, _encoding = undefined) {
            startFileRead(this, blob, (source) => decodeBytes(source.__bytes));
        }

        readAsArrayBuffer(blob) {
            startFileRead(this, blob, (source) => Uint8Array.from(source.__bytes).buffer);
        }

        readAsDataURL(blob) {
            startFileRead(this, blob, (source) => {
                const mimeType = source.type || "application/octet-stream";
                return `data:${mimeType};base64,${encodeBase64(source.__bytes)}`;
            });
        }
    }

    FileReader.EMPTY = 0;
    FileReader.LOADING = 1;
    FileReader.DONE = 2;
    class ReadableStream {
        constructor(init = {}) {
            this.__bodyState = init.__bodyState ?? createBodyState([]);
        }

        get locked() {
            return Boolean(this.__bodyState.readerLocked);
        }

        getReader() {
            if (this.locked) {
                throw new TypeError("ReadableStream is already locked.");
            }

            this.__bodyState.readerLocked = true;
            const state = this.__bodyState;

            return {
                read() {
                    if (state.disturbed) {
                        return Promise.resolve({ value: undefined, done: true });
                    }

                    state.disturbed = true;
                    if (state.owner) state.owner.bodyUsed = true;
                    const chunk = Uint8Array.from(state.bytes);
                    return Promise.resolve(
                        chunk.length === 0
                            ? { value: undefined, done: true }
                            : { value: chunk, done: false },
                    );
                },
                releaseLock() {
                    state.readerLocked = false;
                },
                cancel() {
                    state.disturbed = true;
                    if (state.owner) state.owner.bodyUsed = true;
                    state.readerLocked = false;
                    return Promise.resolve();
                },
            };
        }

        cancel() {
            if (this.locked) {
                return Promise.reject(new TypeError("ReadableStream is locked."));
            }
            return this.getReader().cancel();
        }
        tee() {
            if (this.locked || this.__bodyState.disturbed) {
                throw new TypeError("ReadableStream has already been read.");
            }
            this.__bodyState.readerLocked = true;
            const bytes = this.__bodyState.bytes.slice();
            return [new ReadableStream({ __bodyState: createBodyState(bytes) }), new ReadableStream({ __bodyState: createBodyState(bytes) })];
        }
    }

    const objectUrlRegistry = new Map();

    function startFileRead(reader, blob, read) {
        if (!(blob instanceof Blob)) {
            throw new TypeError("Expected Blob or File");
        }

        if (reader.readyState === FileReader.LOADING) {
            throw new TypeError("InvalidStateError");
        }

        reader.__aborted = false;
        reader.error = null;
        reader.result = null;
        reader.readyState = FileReader.LOADING;
        dispatchFileReaderEvent(reader, "loadstart", blob.size, blob.size);

        queueMicrotask(() => {
            if (reader.__aborted) {
                return;
            }

            try {
                reader.result = read(blob);
                reader.readyState = FileReader.DONE;
                dispatchFileReaderEvent(reader, "progress", blob.size, blob.size);
                dispatchFileReaderEvent(reader, "load", blob.size, blob.size);
                dispatchFileReaderEvent(reader, "loadend", blob.size, blob.size);
            } catch (error) {
                reader.error = error;
                reader.readyState = FileReader.DONE;
                dispatchFileReaderEvent(reader, "error", 0, blob.size);
                dispatchFileReaderEvent(reader, "loadend", 0, blob.size);
            }
        });
    }

    function dispatchFileReaderEvent(reader, type, loaded = 0, total = 0) {
        const event = new Event(type);
        event.loaded = loaded;
        event.total = total;
        event.lengthComputable = true;

        const handler = reader[`on${type}`];
        if (typeof handler === "function") {
            handler.call(reader, event);
        }

        reader.dispatchEvent(event);
    }

    function createBodyState(bytes) {
        return {
            bytes: bytes.slice(),
            disturbed: false,
            readerLocked: false,
            owner: null,
        };
    }

    function attachBodyState(target, bytes, init = {}) {
        const bodyState = createBodyState(bytes);
        bodyState.owner = target;
        target.__bodyState = bodyState;
        target.body = init.nullBody ? null : new ReadableStream({ __bodyState: bodyState });
    }

    function hasBodyValue(value) {
        return value !== undefined && value !== null;
    }

    function consumeBody(target, reader) {
        if (target.bodyUsed || target.__bodyState?.readerLocked) {
            return Promise.reject(new TypeError("Body has already been read."));
        }

        target.__bodyState.disturbed = true;
        target.bodyUsed = true;
        return Promise.resolve().then(() => reader(target.__bodyState.bytes.slice()));
    }

    function flattenParts(parts) {
        const bytes = [];

        for (const part of parts) {
            if (part instanceof Blob) {
                bytes.push(...part.__bytes);
            } else if (part instanceof Uint8Array) {
                bytes.push(...part);
            } else if (part instanceof ArrayBuffer) {
                bytes.push(...new Uint8Array(part));
            } else if (Array.isArray(part)) {
                bytes.push(...part);
            } else {
                bytes.push(...new TextEncoder().encode(String(part)));
            }
        }

        return bytes;
    }

    function normalizeBody(body, headers) {
        if (body === undefined || body === null) {
            return [];
        }

        if (body instanceof Uint8Array) {
            return Array.from(body);
        }

        if (body instanceof ArrayBuffer) {
            return Array.from(new Uint8Array(body));
        }

        if (Array.isArray(body) && body.every((value) => typeof value === "number")) {
            return body.map((value) => Number(value) & 0xff);
        }

        if (body instanceof Blob) {
            if (body.type && !headers.has("content-type")) {
                headers.set("content-type", body.type);
            }
            return body.__bytes.slice();
        }

        if (body instanceof FormData) {
            return serializeFormData(body, headers);
        }

        if (body instanceof URLSearchParams) {
            if (!headers.has("content-type")) {
                headers.set("content-type", "application/x-www-form-urlencoded;charset=UTF-8");
            }
            return Array.from(new TextEncoder().encode(body.toString()));
        }

        if (typeof body === "string") {
            if (!headers.has("content-type")) {
                headers.set("content-type", "text/plain;charset=UTF-8");
            }
            return Array.from(new TextEncoder().encode(body));
        }

        return Array.from(new TextEncoder().encode(String(body)));
    }

    function decodeBytes(bytes) {
        return new TextDecoder().decode(Uint8Array.from(bytes));
    }

    function makeBlobFromBody(bytes, headers) {
        return new Blob([bytes.slice()], { type: getBodyMimeType(headers) });
    }

    function getBodyMimeType(headers) {
        return String(headers.get("content-type") ?? "")
            .split(";")[0]
            .trim()
            .toLowerCase();
    }

    function parseBodyAsFormData(bytes, headers) {
        const contentType = String(headers.get("content-type") ?? "");
        const normalizedContentType = contentType.toLowerCase();

        if (normalizedContentType.startsWith("application/x-www-form-urlencoded")) {
            return createFormDataFromSearchParams(decodeBytes(bytes));
        }

        if (normalizedContentType.startsWith("multipart/form-data")) {
            return parseMultipartFormData(bytes, contentType);
        }

        throw new TypeError(
            `Unable to parse body as FormData for content-type: ${contentType || "unknown"}`,
        );
    }

    function createFormDataFromSearchParams(source) {
        const formData = new FormData();
        const params = new URLSearchParams(source);

        for (const [name, value] of params) {
            formData.append(name, value);
        }

        return formData;
    }

    function serializeFormData(formData, headers) {
        const boundary = `----romformdata${createObjectUrlId()}`;
        const chunks = [];

        for (const entry of formData.__entries) {
            chunks.push(`--${boundary}\r\n`);
            chunks.push(
                `Content-Disposition: form-data; name="${escapeMultipartValue(entry.name)}"${buildMultipartFilename(entry)}\r\n`,
            );

            if (entry.value instanceof Blob) {
                const contentType = entry.value.type || "application/octet-stream";
                chunks.push(`Content-Type: ${contentType}\r\n`);
            }

            chunks.push("\r\n");
            chunks.push(entry.value instanceof Blob ? entry.value.__bytes.slice() : String(entry.value));
            chunks.push("\r\n");
        }

        chunks.push(`--${boundary}--`);

        if (!headers.has("content-type")) {
            headers.set("content-type", `multipart/form-data; boundary=${boundary}`);
        }

        return flattenParts(chunks);
    }

    function buildMultipartFilename(entry) {
        if (!(entry.value instanceof Blob)) {
            return "";
        }

        const filename = entry.filename ?? (entry.value instanceof File ? entry.value.name : "blob");
        return `; filename="${escapeMultipartValue(filename)}"`;
    }

    function escapeMultipartValue(value) {
        return String(value).replace(/["\r\n]/g, "");
    }

    function parseMultipartFormData(bytes, contentType) {
        const boundaryMatch = contentType.match(/boundary=([^;]+)/);

        if (!boundaryMatch) {
            throw new TypeError("Unable to parse multipart form data without boundary.");
        }

        const boundary = boundaryMatch[1].trim().replace(/^"|"$/g, "");
        const source = decodeBytes(bytes);
        const formData = new FormData();
        const parts = source.split(`--${boundary}`);

        for (const rawPart of parts) {
            const part = rawPart.trim();
            if (!part || part === "--") {
                continue;
            }

            const [headerText, ...bodyParts] = part.split("\r\n\r\n");
            if (!headerText || bodyParts.length === 0) {
                continue;
            }

            const bodyText = bodyParts.join("\r\n\r\n").replace(/\r\n$/, "");
            const headersByName = new Map();

            for (const line of headerText.split("\r\n")) {
                const separatorIndex = line.indexOf(":");
                if (separatorIndex === -1) {
                    continue;
                }

                const name = line.slice(0, separatorIndex).trim().toLowerCase();
                const value = line.slice(separatorIndex + 1).trim();
                headersByName.set(name, value);
            }

            const disposition = headersByName.get("content-disposition") ?? "";
            const nameMatch = disposition.match(/name="([^"]*)"/);
            if (!nameMatch) {
                continue;
            }

            const filenameMatch = disposition.match(/filename="([^"]*)"/);
            const partContentType = headersByName.get("content-type") ?? "";

            if (filenameMatch) {
                formData.append(
                    nameMatch[1],
                    new File([bodyText], filenameMatch[1], {
                        type: partContentType,
                    }),
                    filenameMatch[1],
                );
                continue;
            }

            formData.append(nameMatch[1], bodyText);
        }

        return formData;
    }

    function createObjectUrlId() {
        return typeof crypto.randomUUID === "function"
            ? crypto.randomUUID()
            : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
    }

    function encodeBase64(bytes) {
        const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let output = "";

        for (let index = 0; index < bytes.length; index += 3) {
            const byte1 = bytes[index] ?? 0;
            const byte2 = bytes[index + 1] ?? 0;
            const byte3 = bytes[index + 2] ?? 0;
            const chunk = (byte1 << 16) | (byte2 << 8) | byte3;

            output += alphabet[(chunk >> 18) & 0x3f];
            output += alphabet[(chunk >> 12) & 0x3f];
            output += index + 1 < bytes.length ? alphabet[(chunk >> 6) & 0x3f] : "=";
            output += index + 2 < bytes.length ? alphabet[chunk & 0x3f] : "=";
        }

        return output;
    }
