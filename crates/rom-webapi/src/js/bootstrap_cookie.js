    const cookieJar = createCookieJar();
    const initialCookieStore = ((g.__rom_config ?? {}).fetch ?? {}).cookieStore ?? null;

    function createCookieJar() {
        const cookies = new Map();

        return {
            bindDocumentCookie(targetDocument, targetLocation) {
                const jar = this;
                Object.defineProperty(targetDocument, "cookie", {
                    configurable: true,
                    enumerable: true,
                    get() {
                        return jar.__getDocumentCookie(targetLocation.href);
                    },
                    set(value) {
                        jar.__setCookie(String(value), targetLocation.href, "document");
                    },
                });
            },

            appendRequestCookies(headers, requestUrl, credentialsMode, callerOrigin) {
                if (!this.shouldUseNetworkCookies(requestUrl, credentialsMode, callerOrigin)) {
                    return;
                }

                if (headers.has("cookie")) {
                    return;
                }

                const cookieHeader = this.getCookieHeader(requestUrl, callerOrigin);
                if (cookieHeader) {
                    headers.set("cookie", cookieHeader);
                }
            },

            storeResponseCookies(headerEntries, responseUrl, credentialsMode, callerOrigin) {
                if (!this.shouldUseNetworkCookies(responseUrl, credentialsMode, callerOrigin)) {
                    return;
                }

                for (const entry of headerEntries) {
                    if (String(entry.name).toLowerCase() === "set-cookie") {
                        this.__setCookie(String(entry.value), responseUrl, "network");
                    }
                }
            },

            getCookieHeader(requestUrl, callerOrigin) {
                return this.__getCookiesForUrl(requestUrl, {
                    includeHttpOnly: true,
                    includeNonHttpOnly: true,
                    callerOrigin,
                })
                    .map((cookie) => `${cookie.name}=${cookie.value}`)
                    .join("; ");
            },

            shouldUseNetworkCookies(requestUrl, credentialsMode, callerOrigin) {
                const mode = String(credentialsMode ?? "same-origin");
                if (mode === "omit") {
                    return false;
                }

                if (mode === "include") {
                    return true;
                }

                return new URL(requestUrl).origin === String(callerOrigin);
            },

            __getDocumentCookie(requestUrl) {
                return this.__getCookiesForUrl(requestUrl, {
                    includeHttpOnly: false,
                    includeNonHttpOnly: true,
                    callerOrigin: new URL(requestUrl).origin,
                })
                    .map((cookie) => `${cookie.name}=${cookie.value}`)
                    .join("; ");
            },

            __setCookie(cookieString, requestUrl, source) {
                const parsedUrl = new URL(requestUrl);
                const parts = String(cookieString)
                    .split(";")
                    .map((part) => part.trim())
                    .filter(Boolean);

                if (parts.length === 0) {
                    return;
                }

                const separator = parts[0].indexOf("=");
                if (separator <= 0) {
                    return;
                }

                const name = parts[0].slice(0, separator).trim();
                const value = parts[0].slice(separator + 1);
                const attributes = parseCookieAttributes(parts.slice(1));
                const domain = resolveCookieDomain(parsedUrl.hostname, attributes.domain);

                if (!domain) {
                    return;
                }

                const secure = Boolean(attributes.secure);
                const httpOnly = Boolean(attributes.httponly);
                const hostOnly = attributes.domain === null;
                const path = resolveCookiePath(parsedUrl.pathname, attributes.path);
                const expiresAt = resolveCookieExpiry(attributes);

                if (source === "document" && (httpOnly || name.startsWith("__Http-"))) {
                    return;
                }

                if (secure && parsedUrl.protocol !== "https:") {
                    return;
                }

                if (name.startsWith("__Secure-") && (!secure || parsedUrl.protocol !== "https:")) {
                    return;
                }

                if (
                    name.startsWith("__Host-") &&
                    (!secure || parsedUrl.protocol !== "https:" || !hostOnly || path !== "/")
                ) {
                    return;
                }

                const key = buildCookieKey(name, domain, path, hostOnly);

                if (isExpired(expiresAt)) {
                    cookies.delete(key);
                    return;
                }

                cookies.set(key, {
                    name,
                    value,
                    domain,
                    hostOnly,
                    path,
                    secure,
                    httpOnly,
                    sameSite: normalizeSameSite(attributes.samesite),
                    expiresAt,
                });
            },

            __getCookiesForUrl(requestUrl, options) {
                const parsedUrl = new URL(requestUrl);
                const now = Date.now();
                const result = [];

                for (const [key, cookie] of cookies.entries()) {
                    if (isExpired(cookie.expiresAt, now)) {
                        cookies.delete(key);
                        continue;
                    }

                    if (!matchesCookieDomain(parsedUrl.hostname, cookie)) {
                        continue;
                    }

                    if (!matchesCookiePath(parsedUrl.pathname, cookie.path)) {
                        continue;
                    }

                    if (cookie.secure && parsedUrl.protocol !== "https:") {
                        continue;
                    }

                    if (cookie.httpOnly && !options.includeHttpOnly) {
                        continue;
                    }

                    if (!cookie.httpOnly && !options.includeNonHttpOnly) {
                        continue;
                    }

                    if (!isSameSiteAllowed(cookie.sameSite, parsedUrl.origin !== options.callerOrigin)) {
                        continue;
                    }

                    result.push(cookie);
                }

                return result.sort((left, right) => right.path.length - left.path.length);
            },

            importState(serializedState) {
                if (serializedState === null || serializedState === undefined || serializedState === "") {
                    return;
                }

                let entries;
                try {
                    entries = JSON.parse(String(serializedState));
                } catch {
                    return;
                }

                if (!Array.isArray(entries)) {
                    return;
                }

                for (const entry of entries) {
                    const cookie = normalizeCookieEntry(entry);
                    if (!cookie || isExpired(cookie.expiresAt)) {
                        continue;
                    }

                    cookies.set(
                        buildCookieKey(cookie.name, cookie.domain, cookie.path, cookie.hostOnly),
                        cookie,
                    );
                }
            },

            exportState() {
                const now = Date.now();
                const result = [];

                for (const [key, cookie] of cookies.entries()) {
                    if (isExpired(cookie.expiresAt, now)) {
                        cookies.delete(key);
                        continue;
                    }

                    result.push({
                        name: cookie.name,
                        value: cookie.value,
                        domain: cookie.domain,
                        hostOnly: cookie.hostOnly,
                        path: cookie.path,
                        secure: cookie.secure,
                        httpOnly: cookie.httpOnly,
                        sameSite: cookie.sameSite,
                        expiresAt: cookie.expiresAt,
                    });
                }

                return JSON.stringify(result);
            },
        };
    }

    function parseCookieAttributes(parts) {
        const attributes = {
            domain: null,
            path: null,
            samesite: null,
            secure: false,
            httponly: false,
            expires: null,
            "max-age": null,
        };

        for (const part of parts) {
            const separator = part.indexOf("=");
            const key = (separator === -1 ? part : part.slice(0, separator)).trim().toLowerCase();
            const value = separator === -1 ? "" : part.slice(separator + 1).trim();

            if (key === "secure" || key === "httponly") {
                attributes[key] = true;
                continue;
            }

            if (key in attributes) {
                attributes[key] = value;
            }
        }

        return attributes;
    }

    function buildCookieKey(name, domain, path, hostOnly) {
        return `${name};${domain};${path};${hostOnly ? "host" : "domain"}`;
    }

    function resolveCookieDomain(hostname, domainAttribute) {
        if (domainAttribute === null) {
            return String(hostname).toLowerCase();
        }

        const normalizedHost = String(hostname).toLowerCase();
        const normalizedDomain = String(domainAttribute).replace(/^\./, "").toLowerCase();

        if (
            normalizedHost !== normalizedDomain &&
            !normalizedHost.endsWith(`.${normalizedDomain}`)
        ) {
            return null;
        }

        return normalizedDomain;
    }

    function resolveCookiePath(pathname, pathAttribute) {
        if (pathAttribute && String(pathAttribute).startsWith("/")) {
            return String(pathAttribute);
        }

        const normalizedPath = String(pathname || "/");
        if (!normalizedPath.startsWith("/") || normalizedPath === "/") {
            return "/";
        }

        const lastSlash = normalizedPath.lastIndexOf("/");
        return lastSlash <= 0 ? "/" : normalizedPath.slice(0, lastSlash + 1);
    }

    function resolveCookieExpiry(attributes) {
        if (attributes["max-age"] !== null) {
            const seconds = Number(attributes["max-age"]);
            if (!Number.isFinite(seconds)) {
                return null;
            }
            return Date.now() + seconds * 1000;
        }

        if (attributes.expires !== null) {
            const timestamp = Date.parse(attributes.expires);
            return Number.isFinite(timestamp) ? timestamp : null;
        }

        return null;
    }

    function normalizeSameSite(value) {
        const normalized = String(value ?? "").toLowerCase();
        if (normalized === "strict") {
            return "Strict";
        }
        if (normalized === "none") {
            return "None";
        }
        return "Lax";
    }

    function matchesCookieDomain(hostname, cookie) {
        const normalizedHost = String(hostname).toLowerCase();
        if (cookie.hostOnly) {
            return normalizedHost === cookie.domain;
        }
        return normalizedHost === cookie.domain || normalizedHost.endsWith(`.${cookie.domain}`);
    }

    function matchesCookiePath(requestPath, cookiePath) {
        const normalizedRequestPath = String(requestPath || "/");
        const normalizedCookiePath = String(cookiePath || "/");

        if (normalizedCookiePath === "/") {
            return true;
        }

        if (!normalizedRequestPath.startsWith(normalizedCookiePath)) {
            return false;
        }

        return (
            normalizedRequestPath.length === normalizedCookiePath.length ||
            normalizedCookiePath.endsWith("/") ||
            normalizedRequestPath.charAt(normalizedCookiePath.length) === "/"
        );
    }

    function isSameSiteAllowed(sameSite, isCrossSite) {
        if (!isCrossSite) {
            return true;
        }

        return sameSite === "None";
    }

    function isExpired(expiresAt, now = Date.now()) {
        return expiresAt !== null && expiresAt <= now;
    }

    function normalizeCookieEntry(entry) {
        if (!entry || typeof entry !== "object") {
            return null;
        }

        const name = String(entry.name ?? "");
        if (!name) {
            return null;
        }

        const domain = String(entry.domain ?? "").toLowerCase();
        if (!domain) {
            return null;
        }

        const path = String(entry.path ?? "/");
        if (!path.startsWith("/")) {
            return null;
        }

        return {
            name,
            value: String(entry.value ?? ""),
            domain,
            hostOnly: Boolean(entry.hostOnly),
            path,
            secure: Boolean(entry.secure),
            httpOnly: Boolean(entry.httpOnly),
            sameSite: normalizeSameSite(entry.sameSite),
            expiresAt: normalizeExpiresAt(entry.expiresAt),
        };
    }

    function normalizeExpiresAt(value) {
        if (value === null || value === undefined) {
            return null;
        }

        const numeric = Number(value);
        return Number.isFinite(numeric) ? numeric : null;
    }

    function bindDocumentCookie(targetDocument, targetLocation) {
        cookieJar.bindDocumentCookie(targetDocument, targetLocation);
    }

    if (initialCookieStore !== null && initialCookieStore !== undefined && initialCookieStore !== "") {
        cookieJar.importState(initialCookieStore);
    }

    g.__rom_export_cookie_store = () => cookieJar.exportState();
