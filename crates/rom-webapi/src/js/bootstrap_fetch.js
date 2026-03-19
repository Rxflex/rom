    class Request {
        constructor(input, init = {}) {
            if (input instanceof Request) {
                validateRequestSourceBodyReuse(input, init);
                this.url = init.url ?? input.url;
                this.method = normalizeRequestMethod(init.method ?? input.method ?? "GET");
                this.headers = new Headers(init.headers ?? input.headers);
                sanitizeRequestHeaders(this.headers);
                this.signal = init.signal ?? input.signal ?? null;
                this.credentials = normalizeRequestCredentials(
                    init.credentials ?? input.credentials ?? "same-origin",
                );
                this.mode = normalizeRequestMode(init.mode ?? input.mode ?? "cors");
                this.redirect = normalizeRequestRedirect(init.redirect ?? input.redirect ?? "follow");
                this.__bodyIsNull = init.body === undefined
                    ? Boolean(input.__bodyIsNull)
                    : !hasBodyValue(init.body);
                this.__bodyBytes = init.body === undefined
                    ? input.__bodyBytes.slice()
                    : normalizeBody(init.body, this.headers);
                validateNoCorsRequestMode(this.method, this.mode);
                validateRequestBody(this.method, this.__bodyIsNull);
                this.bodyUsed = false;
                attachBodyState(this, this.__bodyBytes, { nullBody: this.__bodyIsNull });
                return;
            }

            this.url = new URL(String(input), location.href).href;
            this.method = normalizeRequestMethod(init.method ?? "GET");
            this.headers = new Headers(init.headers);
            sanitizeRequestHeaders(this.headers);
            this.signal = init.signal ?? null;
            this.credentials = normalizeRequestCredentials(init.credentials ?? "same-origin");
            this.mode = normalizeRequestMode(init.mode ?? "cors");
            this.redirect = normalizeRequestRedirect(init.redirect ?? "follow");
            this.__bodyIsNull = !hasBodyValue(init.body);
            this.__bodyBytes = normalizeBody(init.body, this.headers);
            validateNoCorsRequestMode(this.method, this.mode);
            validateRequestBody(this.method, this.__bodyIsNull);
            this.bodyUsed = false;
            attachBodyState(this, this.__bodyBytes, { nullBody: this.__bodyIsNull });
        }

        clone() {
            if (this.bodyUsed || this.__bodyState.readerLocked) {
                throw new TypeError("Failed to execute 'clone' on 'Request': body has already been used.");
            }

            return new Request(this);
        }

        async text() {
            return consumeBody(this, (bytes) => decodeBytes(bytes));
        }

        async arrayBuffer() {
            return consumeBody(this, (bytes) => Uint8Array.from(bytes).buffer);
        }

        async json() {
            return JSON.parse(await this.text());
        }

        async blob() {
            return consumeBody(this, (bytes) => makeBlobFromBody(bytes, this.headers));
        }

        async formData() {
            return consumeBody(this, (bytes) => parseBodyAsFormData(bytes, this.headers));
        }
    }

    class Response {
        constructor(body = [], init = {}) {
            const status = Number(init.status ?? 200);
            const allowStatusZero = Boolean(init.__allowStatusZero);
            if (
                !(allowStatusZero && status === 0) &&
                (!Number.isInteger(status) || status < 200 || status > 599)
            ) {
                throw new RangeError(
                    "Failed to construct 'Response': The status provided is outside the range [200, 599].",
                );
            }

            this.status = status;
            this.statusText = String(init.statusText ?? "");
            this.ok = this.status >= 200 && this.status < 300;
            this.redirected = Boolean(init.__redirected);
            this.url = String(init.__url ?? "");
            this.type = String(init.__type ?? "default");
            this.headers = new Headers(init.headers);
            this.__bodyBytes = normalizeBody(body, this.headers);
            this.bodyUsed = false;
            this.__bodyIsNull = Boolean(init.__nullBody) || !hasBodyValue(body);
            attachBodyState(this, this.__bodyBytes, { nullBody: this.__bodyIsNull });
        }

        clone() {
            if (this.bodyUsed || this.__bodyState.readerLocked) {
                throw new TypeError("Failed to execute 'clone' on 'Response': body has already been used.");
            }

            return new Response(this.__bodyBytes.slice(), {
                status: this.status,
                statusText: this.statusText,
                headers: this.headers,
                __allowStatusZero: this.status === 0,
                __redirected: this.redirected,
                __url: this.url,
                __type: this.type,
                __nullBody: this.__bodyIsNull,
            });
        }

        async text() {
            return consumeBody(this, (bytes) => decodeBytes(bytes));
        }

        async arrayBuffer() {
            return consumeBody(this, (bytes) => Uint8Array.from(bytes).buffer);
        }

        async json() {
            return JSON.parse(await this.text());
        }

        async blob() {
            return consumeBody(this, (bytes) => makeBlobFromBody(bytes, this.headers));
        }

        async formData() {
            return consumeBody(this, (bytes) => parseBodyAsFormData(bytes, this.headers));
        }
    }

    async function fetch(input, init = {}) {
        const request = input instanceof Request ? new Request(input, init) : new Request(input, init);
        const callerOrigin = location.origin;
        const targetOrigin = new URL(request.url).origin;
        const isCrossOrigin = targetOrigin !== callerOrigin;

        if (request.signal?.aborted) {
            return Promise.reject(request.signal.reason ?? new Error("The operation was aborted."));
        }

        if (request.url.startsWith("blob:")) {
            return Promise.resolve().then(() => {
                const entry = objectUrlRegistry.get(request.url);

                if (!entry) {
                    throw new TypeError("Failed to fetch");
                }

                return new Response(entry.bytes.slice(), {
                    status: 200,
                    headers: entry.type ? [["content-type", entry.type]] : [],
                    url: request.url,
                });
            });
        }

        return Promise.resolve().then(() => {
            if (isCrossOrigin) {
                if (request.mode === "same-origin") {
                    throw new TypeError("Failed to fetch");
                }

                if (request.mode === "no-cors") {
                    const response = performNetworkFetch(request, callerOrigin, {
                        includeCookies: request.credentials === "include",
                    });
                    if (handleRedirectMode(response, request)) {
                        return createOpaqueRedirectResponse();
                    }
                    return createOpaqueResponse();
                }

                if (request.mode !== "cors") {
                    throw new TypeError(`Unsupported Request.mode: ${request.mode}`);
                }

                if (requiresCorsPreflight(request)) {
                    performCorsPreflight(request, callerOrigin);
                }

                const response = performNetworkFetch(request, callerOrigin, {
                    includeCookies: true,
                });
                if (handleRedirectMode(response, request)) {
                    return createOpaqueRedirectResponse();
                }
                validateCorsActualResponse(response, request, callerOrigin);

                cookieJar.storeResponseCookies(
                    response.headers,
                    response.url,
                    request.credentials,
                    callerOrigin,
                );

                return buildCorsResponse(response);
            }

            const response = performNetworkFetch(request, callerOrigin, {
                includeCookies: true,
            });
            if (handleRedirectMode(response, request)) {
                return createOpaqueRedirectResponse();
            }
            cookieJar.storeResponseCookies(
                response.headers,
                response.url,
                request.credentials,
                callerOrigin,
            );
            return buildBasicResponse(response);
        });
    }

    function performNetworkFetch(request, callerOrigin, options = {}) {
        const headers = new Headers(request.headers);
        if (new URL(request.url).origin !== callerOrigin) {
            headers.set("origin", callerOrigin);
        }
        if (options.includeCookies) {
            cookieJar.appendRequestCookies(
                headers,
                request.url,
                request.credentials,
                callerOrigin,
            );
        }

        return JSON.parse(
            g.__rom_fetch_sync(
                JSON.stringify({
                    url: request.url,
                    method: request.method,
                    redirect_mode: request.redirect,
                    headers: headers.entries().map(([name, value]) => ({ name, value })),
                    body: request.__bodyBytes,
                }),
            ),
        );
    }

    function handleRedirectMode(response, request) {
        if (!response.is_redirect_response) {
            return false;
        }

        if (request.redirect === "error") {
            throw new TypeError("Failed to fetch");
        }

        return request.redirect === "manual";
    }

    function performCorsPreflight(request, callerOrigin) {
        const headers = new Headers({
            origin: callerOrigin,
            "access-control-request-method": request.method,
        });
        const unsafeHeaders = getCorsUnsafeHeaderNames(request.headers);

        if (unsafeHeaders.length > 0) {
            headers.set("access-control-request-headers", unsafeHeaders.join(", "));
        }

        const response = JSON.parse(
            g.__rom_fetch_sync(
                JSON.stringify({
                    url: request.url,
                    method: "OPTIONS",
                    headers: headers.entries().map(([name, value]) => ({ name, value })),
                    body: [],
                }),
            ),
        );

        if (response.status < 200 || response.status >= 300) {
            throw new TypeError("Failed to fetch");
        }

        validateCorsPreflightResponse(response, request, callerOrigin, unsafeHeaders);
    }

    function validateCorsPreflightResponse(response, request, callerOrigin, unsafeHeaders) {
        const headerMap = createHeaderMap(response.headers);

        if (!isCorsOriginAllowed(headerMap.get("access-control-allow-origin"), callerOrigin, request.credentials)) {
            throw new TypeError("Failed to fetch");
        }

        if (!isCorsCredentialsAllowed(headerMap, request.credentials, callerOrigin)) {
            throw new TypeError("Failed to fetch");
        }

        const allowedMethods = splitHeaderTokens(headerMap.get("access-control-allow-methods"));
        if (
            allowedMethods.length > 0 &&
            !allowedMethods.includes(request.method.toLowerCase())
        ) {
            throw new TypeError("Failed to fetch");
        }

        if (unsafeHeaders.length > 0) {
            const allowedHeaders = splitHeaderTokens(headerMap.get("access-control-allow-headers"));
            if (allowedHeaders.length === 0) {
                throw new TypeError("Failed to fetch");
            }

            for (const headerName of unsafeHeaders) {
                if (!allowedHeaders.includes(headerName)) {
                    throw new TypeError("Failed to fetch");
                }
            }
        }
    }

    function validateCorsActualResponse(response, request, callerOrigin) {
        const headerMap = createHeaderMap(response.headers);

        if (!isCorsOriginAllowed(headerMap.get("access-control-allow-origin"), callerOrigin, request.credentials)) {
            throw new TypeError("Failed to fetch");
        }

        if (!isCorsCredentialsAllowed(headerMap, request.credentials, callerOrigin)) {
            throw new TypeError("Failed to fetch");
        }
    }

    function isCorsOriginAllowed(allowOrigin, callerOrigin, credentialsMode) {
        if (!allowOrigin) {
            return false;
        }

        if (allowOrigin === "*") {
            return credentialsMode !== "include";
        }

        return allowOrigin === callerOrigin;
    }

    function isCorsCredentialsAllowed(headerMap, credentialsMode, callerOrigin) {
        if (credentialsMode !== "include") {
            return true;
        }

        return (
            headerMap.get("access-control-allow-origin") === callerOrigin &&
            headerMap.get("access-control-allow-credentials") === "true"
        );
    }

    function requiresCorsPreflight(request) {
        if (!isCorsSafelistedMethod(request.method)) {
            return true;
        }

        return getCorsUnsafeHeaderNames(request.headers).length > 0;
    }

    function getCorsUnsafeHeaderNames(headers) {
        const names = [];

        for (const [name, value] of headers) {
            const normalized = String(name).toLowerCase();

            if (normalized === "origin" || normalized === "cookie") {
                continue;
            }

            if (!isCorsSafelistedHeader(normalized, value)) {
                names.push(normalized);
            }
        }

        return Array.from(new Set(names)).sort();
    }

    function isCorsSafelistedMethod(method) {
        return method === "GET" || method === "HEAD" || method === "POST";
    }

    function isCorsSafelistedHeader(name, value) {
        if (name === "accept" || name === "accept-language" || name === "content-language") {
            return true;
        }

        if (name === "content-type") {
            const mimeType = String(value).split(";")[0].trim().toLowerCase();
            return (
                mimeType === "application/x-www-form-urlencoded" ||
                mimeType === "multipart/form-data" ||
                mimeType === "text/plain"
            );
        }

        return false;
    }

    function buildBasicResponse(response) {
        const headers = response.headers.filter((entry) =>
            !isForbiddenResponseHeader(entry.name),
        );

        return new Response(response.body, {
            status: response.status,
            statusText: response.status_text,
            headers: headers.map((entry) => [entry.name, entry.value]),
            __redirected: response.redirected,
            __url: response.url,
            __type: "basic",
        });
    }

    function buildCorsResponse(response) {
        const headerMap = createHeaderMap(response.headers);
        const exposedHeaders = new Set(splitHeaderTokens(headerMap.get("access-control-expose-headers")));
        const headers = response.headers.filter((entry) =>
            isCorsExposedHeader(entry.name, exposedHeaders),
        );

        return new Response(response.body, {
            status: response.status,
            statusText: response.status_text,
            headers: headers.map((entry) => [entry.name, entry.value]),
            __redirected: response.redirected,
            __url: response.url,
            __type: "cors",
        });
    }

    function createOpaqueResponse() {
        return new Response([], {
            status: 0,
            statusText: "",
            headers: [],
            __allowStatusZero: true,
            __redirected: false,
            __url: "",
            __type: "opaque",
            __nullBody: true,
        });
    }

    function createOpaqueRedirectResponse() {
        return new Response([], {
            status: 0,
            statusText: "",
            headers: [],
            __allowStatusZero: true,
            __redirected: false,
            __url: "",
            __type: "opaqueredirect",
            __nullBody: true,
        });
    }

    function createHeaderMap(headers) {
        const headerMap = new Map();

        for (const entry of headers) {
            const name = String(entry.name).toLowerCase();
            const current = headerMap.get(name);
            if (current === undefined) {
                headerMap.set(name, String(entry.value));
                continue;
            }
            headerMap.set(name, `${current}, ${entry.value}`);
        }

        return headerMap;
    }

    function splitHeaderTokens(value) {
        if (!value) {
            return [];
        }

        return String(value)
            .split(",")
            .map((token) => token.trim().toLowerCase())
            .filter(Boolean);
    }

    function isCorsExposedHeader(name, exposedHeaders) {
        const normalized = String(name).toLowerCase();
        if (isForbiddenResponseHeader(normalized)) {
            return false;
        }

        return (
            normalized === "cache-control" ||
            normalized === "content-language" ||
            normalized === "content-length" ||
            normalized === "content-type" ||
            normalized === "expires" ||
            normalized === "last-modified" ||
            normalized === "pragma" ||
            exposedHeaders.has(normalized)
        );
    }

    function sanitizeRequestHeaders(headers) {
        for (const [name] of headers) {
            if (isForbiddenRequestHeader(name)) {
                headers.delete(name);
            }
        }
    }

    function isForbiddenRequestHeader(name) {
        const normalized = String(name).toLowerCase();
        return normalized === "cookie" || normalized === "cookie2";
    }

    function isForbiddenResponseHeader(name) {
        const normalized = String(name).toLowerCase();
        return normalized === "set-cookie" || normalized === "set-cookie2";
    }

    function validateRequestBody(method, isNullBody) {
        if (isNullBody) {
            return;
        }

        if (method === "GET" || method === "HEAD") {
            throw new TypeError(
                "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
            );
        }
    }

    function validateRequestSourceBodyReuse(input, init) {
        if (init.body !== undefined) {
            return;
        }

        if (input.bodyUsed || input.__bodyState?.readerLocked) {
            throw new TypeError(
                "Failed to construct 'Request': Cannot construct a Request from a Request with a used body.",
            );
        }
    }

    function normalizeRequestMethod(method) {
        const value = String(method ?? "GET");
        if (!/^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/.test(value)) {
            throw new TypeError(`Failed to construct 'Request': '${value}' is not a valid HTTP method.`);
        }

        const normalizedUppercase = value.toUpperCase();
        if (
            normalizedUppercase === "CONNECT" ||
            normalizedUppercase === "TRACE" ||
            normalizedUppercase === "TRACK"
        ) {
            throw new TypeError(
                `Failed to construct 'Request': '${value}' HTTP method is unsupported.`,
            );
        }

        if (
            normalizedUppercase === "DELETE" ||
            normalizedUppercase === "GET" ||
            normalizedUppercase === "HEAD" ||
            normalizedUppercase === "OPTIONS" ||
            normalizedUppercase === "POST" ||
            normalizedUppercase === "PUT"
        ) {
            return normalizedUppercase;
        }

        return value;
    }

    function normalizeRequestCredentials(credentials) {
        const value = String(credentials ?? "same-origin");
        if (value === "omit" || value === "same-origin" || value === "include") {
            return value;
        }

        throw new TypeError(
            `Failed to construct 'Request': '${value}' is not a valid enum value of type RequestCredentials.`,
        );
    }

    function normalizeRequestMode(mode) {
        const value = String(mode ?? "cors");
        if (value === "cors" || value === "no-cors" || value === "same-origin") {
            return value;
        }

        throw new TypeError(
            `Failed to construct 'Request': '${value}' is not a valid enum value of type RequestMode.`,
        );
    }

    function normalizeRequestRedirect(redirect) {
        const value = String(redirect ?? "follow");
        if (value === "follow" || value === "error" || value === "manual") {
            return value;
        }

        throw new TypeError(
            `Failed to construct 'Request': '${value}' is not a valid enum value of type RequestRedirect.`,
        );
    }

    function validateNoCorsRequestMode(method, mode) {
        if (mode !== "no-cors") {
            return;
        }

        if (!isCorsSafelistedMethod(method)) {
            throw new TypeError(
                `Failed to construct 'Request': '${method}' is unsupported in no-cors mode.`,
            );
        }
    }

    URL.createObjectURL = (object) => {
        if (!(object instanceof Blob)) {
            throw new TypeError(
                "Failed to execute 'createObjectURL' on 'URL': parameter 1 is not of type 'Blob'.",
            );
        }

        const objectUrl = `blob:${location.origin}/${createObjectUrlId()}`;
        objectUrlRegistry.set(objectUrl, {
            bytes: object.__bytes.slice(),
            type: object.type,
        });
        return objectUrl;
    };

    URL.revokeObjectURL = (objectUrl) => {
        objectUrlRegistry.delete(String(objectUrl));
    };
