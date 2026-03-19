    class EventSource extends EventTarget {
        constructor(url, init = {}) {
            super();
            this.url = parseEventSourceUrl(url);
            this.withCredentials = Boolean(init.withCredentials);
            this.readyState = EventSource.CONNECTING;
            this.onopen = null;
            this.onmessage = null;
            this.onerror = null;
            this.__lastEventId = "";
            this.__retry = 3000;
            this.__controller = null;
            this.__reconnectTimer = null;
            this.__connect();
        }

        close() {
            if (this.readyState === EventSource.CLOSED) {
                return;
            }

            this.readyState = EventSource.CLOSED;
            if (this.__reconnectTimer !== null) {
                clearTimeout(this.__reconnectTimer);
                this.__reconnectTimer = null;
            }
            if (this.__controller) {
                this.__controller.abort(new Error("EventSource closed"));
                this.__controller = null;
            }
        }

        async __connect() {
            const controller = new AbortController();
            this.__controller = controller;

            try {
                const headers = { accept: "text/event-stream" };
                if (this.__lastEventId) {
                    headers["last-event-id"] = this.__lastEventId;
                }

                const response = await fetch(this.url, {
                    headers,
                    credentials: this.withCredentials ? "include" : "same-origin",
                    signal: controller.signal,
                });

                if (!response.ok) {
                    this.__failConnection(controller);
                    return;
                }

                const contentType = response.headers.get("content-type") ?? "";
                if (!contentType.toLowerCase().includes("text/event-stream")) {
                    this.__failConnection(controller);
                    return;
                }

                if (this.readyState === EventSource.CLOSED || this.__controller !== controller) {
                    return;
                }

                this.readyState = EventSource.OPEN;
                emitEventSourceEvent(this, "open", new Event("open"));

                const text = await response.text();
                const origin = new URL(this.url).origin;
                const entries = parseEventStream(text);

                for (const entry of entries) {
                    if (this.readyState === EventSource.CLOSED || this.__controller !== controller) {
                        return;
                    }

                    if (entry.retry !== null) {
                        this.__retry = entry.retry;
                    }

                    if (entry.id !== null) {
                        this.__lastEventId = entry.id;
                    }

                    if (!entry.data.length) {
                        continue;
                    }

                    const type = entry.event || "message";
                    emitEventSourceEvent(
                        this,
                        type,
                        new MessageEvent(type, {
                            data: entry.data.join("\n"),
                            origin,
                            lastEventId: this.__lastEventId,
                            source: null,
                            ports: [],
                        }),
                    );
                }

                this.__scheduleReconnect(controller);
            } catch (error) {
                if (this.readyState === EventSource.CLOSED || this.__controller !== controller) {
                    return;
                }

                this.__scheduleReconnect(controller);
            } finally {
                if (this.__controller === controller && this.readyState !== EventSource.CONNECTING) {
                    this.__controller = null;
                }
            }
        }

        __scheduleReconnect(controller) {
            if (this.readyState === EventSource.CLOSED || this.__controller !== controller) {
                return;
            }

            this.__controller = null;

            queueMicrotask(() => {
                if (this.readyState === EventSource.CLOSED) {
                    return;
                }

                this.readyState = EventSource.CONNECTING;
                emitEventSourceEvent(this, "error", new Event("error"));

                if (this.__reconnectTimer !== null) {
                    clearTimeout(this.__reconnectTimer);
                }

                this.__reconnectTimer = setTimeout(() => {
                    this.__reconnectTimer = null;
                    if (this.readyState === EventSource.CLOSED) {
                        return;
                    }

                    this.__connect();
                }, this.__retry);
            });
        }

        __failConnection(controller) {
            if (this.readyState === EventSource.CLOSED || this.__controller !== controller) {
                return;
            }

            this.__controller = null;

            queueMicrotask(() => {
                if (this.readyState === EventSource.CLOSED) {
                    return;
                }

                this.readyState = EventSource.CLOSED;
                emitEventSourceEvent(this, "error", new Event("error"));
            });
        }
    }

    EventSource.CONNECTING = 0;
    EventSource.OPEN = 1;
    EventSource.CLOSED = 2;
    EventSource.prototype.CONNECTING = EventSource.CONNECTING;
    EventSource.prototype.OPEN = EventSource.OPEN;
    EventSource.prototype.CLOSED = EventSource.CLOSED;

    function emitEventSourceEvent(source, type, event) {
        queueMicrotask(() => {
            const handlerName = `on${type}`;
            if (typeof source[handlerName] === "function") {
                source[handlerName](event);
            }

            source.dispatchEvent(event);
        });
    }

    function parseEventStream(input) {
        const entries = [];
        let current = createEventSourceEntry();

        for (const rawLine of String(input).split(/\r?\n/)) {
            if (rawLine === "") {
                flushEventSourceEntry(entries, current);
                current = createEventSourceEntry();
                continue;
            }

            if (rawLine.startsWith(":")) {
                continue;
            }

            const separatorIndex = rawLine.indexOf(":");
            const field = separatorIndex === -1 ? rawLine : rawLine.slice(0, separatorIndex);
            let value = separatorIndex === -1 ? "" : rawLine.slice(separatorIndex + 1);
            if (value.startsWith(" ")) {
                value = value.slice(1);
            }

            switch (field) {
                case "data":
                    current.data.push(value);
                    break;
                case "event":
                    current.event = value;
                    break;
                case "id":
                    current.id = value;
                    break;
                case "retry":
                    current.retry = Number.parseInt(value, 10);
                    break;
                default:
                    break;
            }
        }

        flushEventSourceEntry(entries, current);
        return entries;
    }

    function createEventSourceEntry() {
        return {
            data: [],
            event: "",
            id: null,
            retry: null,
        };
    }

    function flushEventSourceEntry(entries, entry) {
        if (!entry.data.length && !entry.event && entry.id === null && entry.retry === null) {
            return;
        }

        entries.push({
            data: entry.data.slice(),
            event: entry.event,
            id: entry.id,
            retry: Number.isFinite(entry.retry) && entry.retry >= 0 ? entry.retry : null,
        });
    }

    function parseEventSourceUrl(url) {
        try {
            return new URL(String(url), location.href).href;
        } catch (error) {
            const syntaxError = new Error("Invalid EventSource URL.");
            syntaxError.name = "SyntaxError";
            throw syntaxError;
        }
    }
