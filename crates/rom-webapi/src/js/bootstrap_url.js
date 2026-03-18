    function parseUrl(input, base = undefined) {
        return JSON.parse(
            g.__rom_parse_url(
                JSON.stringify({
                    input: String(input),
                    base: base === undefined || base === null ? null : String(base),
                }),
            ),
        );
    }

    function encodeSearchParam(value) {
        return encodeURIComponent(String(value)).replace(/%20/g, "+");
    }

    function decodeSearchParam(value) {
        return decodeURIComponent(String(value).replace(/\+/g, " "));
    }

    class URLSearchParams {
        constructor(init = undefined) {
            this.__pairs = [];
            this.__onChange = null;

            if (init === undefined || init === null) {
                return;
            }

            if (init instanceof URLSearchParams) {
                this.__pairs = init.entries();
                return;
            }

            if (typeof init === "string") {
                const source = init.startsWith("?") ? init.slice(1) : init;
                if (!source) {
                    return;
                }
                for (const segment of source.split("&")) {
                    if (!segment) {
                        continue;
                    }
                    const [name, value = ""] = segment.split("=");
                    this.append(decodeSearchParam(name), decodeSearchParam(value));
                }
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

        __attach(onChange) {
            this.__onChange = onChange;
            return this;
        }

        __commit() {
            if (typeof this.__onChange === "function") {
                this.__onChange(this.toString());
            }
        }

        append(name, value) {
            this.__pairs.push([String(name), String(value)]);
            this.__commit();
        }

        delete(name) {
            const normalized = String(name);
            this.__pairs = this.__pairs.filter(([key]) => key !== normalized);
            this.__commit();
        }

        get(name) {
            const normalized = String(name);
            const pair = this.__pairs.find(([key]) => key === normalized);
            return pair ? pair[1] : null;
        }

        getAll(name) {
            const normalized = String(name);
            return this.__pairs
                .filter(([key]) => key === normalized)
                .map(([, value]) => value);
        }

        has(name) {
            const normalized = String(name);
            return this.__pairs.some(([key]) => key === normalized);
        }

        set(name, value) {
            const normalized = String(name);
            const nextValue = String(value);
            let replaced = false;
            const nextPairs = [];

            for (const [key, currentValue] of this.__pairs) {
                if (key !== normalized) {
                    nextPairs.push([key, currentValue]);
                    continue;
                }
                if (!replaced) {
                    nextPairs.push([key, nextValue]);
                    replaced = true;
                }
            }

            if (!replaced) {
                nextPairs.push([normalized, nextValue]);
            }

            this.__pairs = nextPairs;
            this.__commit();
        }

        sort() {
            this.__pairs.sort(([left], [right]) => left.localeCompare(right));
            this.__commit();
        }

        keys() {
            return this.__pairs.map(([key]) => key);
        }

        values() {
            return this.__pairs.map(([, value]) => value);
        }

        entries() {
            return this.__pairs.map(([key, value]) => [key, value]);
        }

        forEach(callback, thisArg = undefined) {
            for (const [key, value] of this.__pairs) {
                callback.call(thisArg, value, key, this);
            }
        }

        toString() {
            return this.__pairs
                .map(([key, value]) => `${encodeSearchParam(key)}=${encodeSearchParam(value)}`)
                .join("&");
        }

        [Symbol.iterator]() {
            return this.entries()[Symbol.iterator]();
        }
    }

    class URL {
        constructor(input, base = undefined) {
            this.__setState(parseUrl(input, base));
        }

        __setState(state) {
            this.__updating = true;
            this._href = state.href;
            this.origin = state.origin;
            this._protocol = state.protocol;
            this._username = state.username;
            this._password = state.password;
            this._host = state.host;
            this._hostname = state.hostname;
            this._port = state.port;
            this._pathname = state.pathname;
            this._search = state.search;
            this._hash = state.hash;
            this.searchParams = new URLSearchParams(this._search).__attach((value) => {
                this.search = value ? `?${value}` : "";
            });
            this.__updating = false;
        }

        __buildHref() {
            const auth = this._username
                ? `${this._username}${this._password ? `:${this._password}` : ""}@`
                : "";
            return `${this._protocol}//${auth}${this._host}${this._pathname}${this._search}${this._hash}`;
        }

        __reparse() {
            if (this.__updating) {
                return;
            }
            this.__setState(parseUrl(this.__buildHref()));
        }

        toString() {
            return this._href;
        }

        toJSON() {
            return this._href;
        }

        set href(value) {
            this.__setState(parseUrl(value));
        }

        get href() {
            return this._href;
        }

        set protocol(value) {
            this._protocol = String(value).endsWith(":") ? String(value) : `${value}:`;
            this.__reparse();
        }

        get protocol() {
            return this._protocol;
        }

        set username(value) {
            this._username = String(value);
            this.__reparse();
        }

        get username() {
            return this._username;
        }

        set password(value) {
            this._password = String(value);
            this.__reparse();
        }

        get password() {
            return this._password;
        }

        set host(value) {
            const parsed = parseUrl(
                `${this._protocol}//${this._username ? `${this._username}${this._password ? `:${this._password}` : ""}@` : ""}${value}${this._pathname}${this._search}${this._hash}`,
            );
            this.__setState(parsed);
        }

        get host() {
            return this._host;
        }

        set hostname(value) {
            this._hostname = String(value);
            this._host = this._port ? `${this._hostname}:${this._port}` : this._hostname;
            this.__reparse();
        }

        get hostname() {
            return this._hostname;
        }

        set port(value) {
            this._port = String(value);
            this._host = this._port ? `${this._hostname}:${this._port}` : this._hostname;
            this.__reparse();
        }

        get port() {
            return this._port;
        }

        set pathname(value) {
            const next = String(value);
            this._pathname = next.startsWith("/") ? next : `/${next}`;
            this.__reparse();
        }

        get pathname() {
            return this._pathname;
        }

        set hash(value) {
            this._hash = value ? (String(value).startsWith("#") ? String(value) : `#${value}`) : "";
            this.__reparse();
        }

        get hash() {
            return this._hash;
        }

        set search(value) {
            this._search = value
                ? (String(value).startsWith("?") ? String(value) : `?${value}`)
                : "";
            this.__reparse();
        }

        get search() {
            return this._search;
        }
    }
