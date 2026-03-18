    class CloseEvent extends Event {
        constructor(type, init = {}) {
            super(type, init);
            this.code = Number(init.code ?? 0);
            this.reason = String(init.reason ?? "");
            this.wasClean = Boolean(init.wasClean);
        }
    }

    class WebSocket extends EventTarget {
        constructor(url, protocols = []) {
            super();
            const protocolList = normalizeWebSocketProtocols(protocols);
            const response = JSON.parse(
                g.__rom_websocket_connect(
                    JSON.stringify({
                        url: new URL(String(url), location.href).href,
                        protocols: protocolList,
                    }),
                ),
            );

            this.url = String(response.url ?? "");
            this.readyState = WebSocket.CONNECTING;
            this.protocol = String(response.protocol ?? "");
            this.extensions = "";
            this.binaryType = "blob";
            this.bufferedAmount = 0;
            this.onopen = null;
            this.onmessage = null;
            this.onerror = null;
            this.onclose = null;
            defineReadOnly(this, "__socketId", String(response.socket_id ?? ""));
            defineReadOnly(this, "__closeDispatched", false);
            this.__pollTimer = null;
            this.__pollPending = false;
            this.readyState = WebSocket.OPEN;
            dispatchWebSocketEvent(this, "open", new Event("open"));
            this.__schedulePoll();
        }

        send(data) {
            if (this.readyState !== WebSocket.OPEN) {
                throw new Error("WebSocket is not open");
            }

            const payload = serializeWebSocketData(data);
            g.__rom_websocket_send(
                JSON.stringify({
                    socket_id: this.__socketId,
                    kind: payload.kind,
                    text: payload.text ?? "",
                    bytes: payload.bytes ?? [],
                }),
            );
            this.__schedulePoll();
        }

        close(code, reason) {
            if (this.readyState === WebSocket.CLOSING || this.readyState === WebSocket.CLOSED) {
                return;
            }

            this.readyState = WebSocket.CLOSING;
            const result = JSON.parse(
                g.__rom_websocket_close(
                    JSON.stringify({
                        socket_id: this.__socketId,
                        code: code ?? null,
                        reason: reason ?? null,
                    }),
                ),
            );
            this.__applyClose(result);
        }

        __schedulePoll(delay = 0) {
            if (
                this.readyState !== WebSocket.OPEN ||
                this.__pollPending ||
                this.__pollTimer !== null
            ) {
                return;
            }

            this.__pollTimer = setTimeout(() => {
                this.__pollTimer = null;

                if (this.readyState !== WebSocket.OPEN) {
                    return;
                }

                this.__pollPending = true;
                const result = JSON.parse(
                    g.__rom_websocket_poll(
                        JSON.stringify({
                            socket_id: this.__socketId,
                        }),
                    ),
                );
                this.__pollPending = false;

                for (const frame of result.messages ?? []) {
                    dispatchWebSocketEvent(
                        this,
                        "message",
                        new MessageEvent("message", {
                            data: deserializeWebSocketData(this.binaryType, frame),
                            origin: new URL(this.url).origin,
                            source: null,
                            ports: [],
                        }),
                    );
                }

                if (result.close_event) {
                    this.__applyClose(result.close_event);
                    return;
                }

                this.__schedulePoll((result.messages ?? []).length === 0 ? 10 : 0);
            }, delay);
        }

        __applyClose(closeEvent) {
            if (this.readyState === WebSocket.CLOSED) {
                return;
            }

            if (this.__pollTimer !== null) {
                clearTimeout(this.__pollTimer);
                this.__pollTimer = null;
            }
            this.__pollPending = false;
            this.readyState = WebSocket.CLOSED;
            dispatchWebSocketEvent(
                this,
                "close",
                new CloseEvent("close", {
                    code: closeEvent.code,
                    reason: closeEvent.reason,
                    wasClean: closeEvent.was_clean,
                }),
            );
        }
    }

    WebSocket.CONNECTING = 0;
    WebSocket.OPEN = 1;
    WebSocket.CLOSING = 2;
    WebSocket.CLOSED = 3;
    WebSocket.prototype.CONNECTING = WebSocket.CONNECTING;
    WebSocket.prototype.OPEN = WebSocket.OPEN;
    WebSocket.prototype.CLOSING = WebSocket.CLOSING;
    WebSocket.prototype.CLOSED = WebSocket.CLOSED;

    function normalizeWebSocketProtocols(protocols) {
        if (typeof protocols === "string") {
            return [protocols];
        }

        if (Array.isArray(protocols)) {
            return protocols.map(String);
        }

        return [];
    }

    function serializeWebSocketData(data) {
        if (typeof data === "string") {
            return { kind: "text", text: data };
        }

        if (data instanceof Blob) {
            return { kind: "binary", bytes: data.__bytes.slice() };
        }

        if (data instanceof ArrayBuffer || ArrayBuffer.isView(data)) {
            return { kind: "binary", bytes: toByteArray(data) };
        }

        throw new TypeError("Unsupported WebSocket payload");
    }

    function deserializeWebSocketData(binaryType, frame) {
        if (frame.kind === "text") {
            return frame.text ?? "";
        }

        const bytes = frame.bytes ?? [];
        if (binaryType === "arraybuffer") {
            return toArrayBuffer(bytes);
        }

        return new Blob([Uint8Array.from(bytes)]);
    }

    function dispatchWebSocketEvent(target, type, event) {
        queueMicrotask(() => {
            const handlerName = `on${type}`;
            if (typeof target[handlerName] === "function") {
                target[handlerName](event);
            }

            target.dispatchEvent(event);
        });
    }
