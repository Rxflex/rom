    class URLPattern {
        constructor(input = {}, baseURLOrOptions = undefined, options = undefined) {
            const resolved = resolveUrlPatternInit(input, baseURLOrOptions, options);
            this.protocol = resolved.protocol;
            this.username = resolved.username;
            this.password = resolved.password;
            this.hostname = resolved.hostname;
            this.port = resolved.port;
            this.pathname = resolved.pathname;
            this.search = resolved.search;
            this.hash = resolved.hash;
            this.ignoreCase = resolved.ignoreCase;
        }

        test(input, baseURL = undefined) {
            return this.exec(input, baseURL) !== null;
        }

        exec(input, baseURL = undefined) {
            const parsed = normalizeUrlPatternInput(input, baseURL);
            const protocol = matchComponent(this.protocol, parsed.protocol, this.ignoreCase);
            const username = matchComponent(this.username, parsed.username, this.ignoreCase);
            const password = matchComponent(this.password, parsed.password, this.ignoreCase);
            const hostname = matchComponent(this.hostname, parsed.hostname, this.ignoreCase);
            const port = matchComponent(this.port, parsed.port, this.ignoreCase);
            const pathname = matchPathname(this.pathname, parsed.pathname, this.ignoreCase);
            const search = matchComponent(this.search, parsed.search, this.ignoreCase);
            const hash = matchComponent(this.hash, parsed.hash, this.ignoreCase);

            if (!protocol || !username || !password || !hostname || !port || !pathname || !search || !hash) {
                return null;
            }

            return {
                inputs: [typeof input === "string" ? String(input) : input],
                protocol,
                username,
                password,
                hostname,
                port,
                pathname,
                search,
                hash,
            };
        }
    }

    function resolveUrlPatternInit(input, baseURLOrOptions, options) {
        let baseURL = null;
        let ignoreCase = false;
        let init = {};

        if (typeof input === "string") {
            if (typeof baseURLOrOptions === "string") {
                baseURL = baseURLOrOptions;
                ignoreCase = Boolean(options?.ignoreCase);
            } else {
                ignoreCase = Boolean(baseURLOrOptions?.ignoreCase);
            }

            const parsed = new URL(input, baseURL ?? location.href);
            init = {
                protocol: parsed.protocol.slice(0, -1),
                username: parsed.username,
                password: parsed.password,
                hostname: parsed.hostname,
                port: parsed.port,
                pathname: parsed.pathname,
                search: parsed.search,
                hash: parsed.hash,
            };
        } else {
            init = { ...input };
            ignoreCase = Boolean(baseURLOrOptions?.ignoreCase);
            baseURL = init.baseURL ?? null;

            if (baseURL) {
                const parsed = new URL(baseURL);
                init.protocol ??= parsed.protocol.slice(0, -1);
                init.username ??= parsed.username;
                init.password ??= parsed.password;
                init.hostname ??= parsed.hostname;
                init.port ??= parsed.port;
                init.pathname ??= parsed.pathname;
                init.search ??= parsed.search;
                init.hash ??= parsed.hash;
            }
        }

        return {
            protocol: normalizePatternComponent(init.protocol, "*"),
            username: normalizePatternComponent(init.username, "*"),
            password: normalizePatternComponent(init.password, "*"),
            hostname: normalizePatternComponent(init.hostname, "*"),
            port: normalizePatternComponent(init.port, "*"),
            pathname: normalizePatternComponent(init.pathname, "*"),
            search: normalizePatternComponent(init.search, "*"),
            hash: normalizePatternComponent(init.hash, "*"),
            ignoreCase,
        };
    }

    function normalizePatternComponent(value, fallback) {
        if (value === undefined || value === null || value === "") {
            return fallback;
        }

        return String(value);
    }

    function normalizeUrlPatternInput(input, baseURL) {
        if (typeof input === "string") {
            const parsed = new URL(input, baseURL ?? location.href);
            return {
                protocol: parsed.protocol.slice(0, -1),
                username: parsed.username,
                password: parsed.password,
                hostname: parsed.hostname,
                port: parsed.port,
                pathname: parsed.pathname,
                search: parsed.search,
                hash: parsed.hash,
            };
        }

        return {
            protocol: String(input.protocol ?? ""),
            username: String(input.username ?? ""),
            password: String(input.password ?? ""),
            hostname: String(input.hostname ?? ""),
            port: String(input.port ?? ""),
            pathname: String(input.pathname ?? "/"),
            search: String(input.search ?? ""),
            hash: String(input.hash ?? ""),
        };
    }

    function matchComponent(pattern, value, ignoreCase) {
        if (pattern === "*") {
            return { input: value, groups: {} };
        }

        const flags = ignoreCase ? "i" : "";
        const expression = new RegExp(`^${escapePattern(pattern).replace(/\\\*/g, ".*")}$`, flags);
        if (!expression.test(String(value))) {
            return null;
        }

        return { input: value, groups: {} };
    }

    function matchPathname(pattern, value, ignoreCase) {
        if (pattern === "*") {
            return { input: value, groups: {} };
        }

        const { expression, groupNames } = compilePathnamePattern(pattern, ignoreCase);
        const match = expression.exec(String(value));
        if (!match) {
            return null;
        }

        const groups = {};
        for (const groupName of groupNames) {
            groups[groupName] = match.groups?.[groupName] ?? "";
        }

        return {
            input: value,
            groups,
        };
    }

    function compilePathnamePattern(pattern, ignoreCase) {
        const groupNames = [];
        let source = "^";

        for (let index = 0; index < pattern.length; index += 1) {
            const char = pattern[index];

            if (char === ":") {
                let cursor = index + 1;
                let name = "";
                while (cursor < pattern.length && /[A-Za-z0-9_]/.test(pattern[cursor])) {
                    name += pattern[cursor];
                    cursor += 1;
                }

                if (!name) {
                    source += ":";
                    continue;
                }

                groupNames.push(name);
                source += `(?<${name}>[^/]+)`;
                index = cursor - 1;
                continue;
            }

            if (char === "*") {
                source += ".*";
                continue;
            }

            source += escapeRegExp(char);
        }

        source += "$";
        return {
            expression: new RegExp(source, ignoreCase ? "i" : ""),
            groupNames,
        };
    }

    function escapePattern(value) {
        return Array.from(String(value), escapeRegExp).join("");
    }

    function escapeRegExp(value) {
        return String(value).replace(/[|\\{}()[\]^$+?.]/g, "\\$&");
    }
