    class TextMetrics {
        constructor(width, fontSize, textLength) {
            const normalizedWidth = Number(width) || 0;
            const normalizedFontSize = Math.max(1, Number(fontSize) || 10);
            const normalizedTextLength = Math.max(0, Number(textLength) || 0);
            const ascent = normalizedFontSize * 0.78;
            const descent = normalizedFontSize * 0.22;

            this.width = normalizedWidth;
            this.actualBoundingBoxLeft = Math.min(normalizedWidth * 0.04, 2);
            this.actualBoundingBoxRight = Math.max(0, normalizedWidth - this.actualBoundingBoxLeft);
            this.actualBoundingBoxAscent = ascent;
            this.actualBoundingBoxDescent = descent;
            this.fontBoundingBoxAscent = normalizedFontSize * 0.82;
            this.fontBoundingBoxDescent = normalizedFontSize * 0.26;
            this.emHeightAscent = normalizedFontSize * 0.8;
            this.emHeightDescent = normalizedFontSize * 0.2;
            this.hangingBaseline = normalizedFontSize * 0.62;
            this.alphabeticBaseline = 0;
            this.ideographicBaseline = -descent * 0.8;
            this.__textLength = normalizedTextLength;
        }

        toJSON() {
            return {
                width: this.width,
                actualBoundingBoxLeft: this.actualBoundingBoxLeft,
                actualBoundingBoxRight: this.actualBoundingBoxRight,
                actualBoundingBoxAscent: this.actualBoundingBoxAscent,
                actualBoundingBoxDescent: this.actualBoundingBoxDescent,
                fontBoundingBoxAscent: this.fontBoundingBoxAscent,
                fontBoundingBoxDescent: this.fontBoundingBoxDescent,
                emHeightAscent: this.emHeightAscent,
                emHeightDescent: this.emHeightDescent,
                hangingBaseline: this.hangingBaseline,
                alphabeticBaseline: this.alphabeticBaseline,
                ideographicBaseline: this.ideographicBaseline,
                textLength: this.__textLength,
            };
        }
    }

    function hashCanvasValue(seed, value) {
        const text = typeof value === "string" ? value : JSON.stringify(value);
        let nextSeed = seed >>> 0;

        for (let index = 0; index < text.length; index += 1) {
            nextSeed ^= text.charCodeAt(index);
            nextSeed = Math.imul(nextSeed, 16777619) >>> 0;
        }

        return nextSeed >>> 0;
    }

    function hashCanvasNumbers(seed, ...values) {
        let nextSeed = seed >>> 0;

        for (const value of values) {
            const normalized = Number.isFinite(Number(value))
                ? Math.trunc(Number(value) * 1024)
                : 0;
            nextSeed ^= normalized >>> 0;
            nextSeed = Math.imul(nextSeed ^ (nextSeed >>> 15), 2246822519) >>> 0;
            nextSeed ^= nextSeed >>> 13;
        }

        return nextSeed >>> 0;
    }

    function finalizeCanvasSeed(seed) {
        let nextSeed = seed >>> 0;
        nextSeed ^= nextSeed >>> 16;
        nextSeed = Math.imul(nextSeed, 2246822519) >>> 0;
        nextSeed ^= nextSeed >>> 13;
        nextSeed = Math.imul(nextSeed, 3266489917) >>> 0;
        nextSeed ^= nextSeed >>> 16;
        return nextSeed >>> 0;
    }

    function normalizeCanvasInteger(value, fallback = 0) {
        const numeric = Number(value);
        if (!Number.isFinite(numeric)) {
            return fallback;
        }
        return Math.trunc(numeric);
    }

    function normalizeCanvasDimension(value, fallback) {
        return Math.max(1, normalizeCanvasInteger(value, fallback));
    }

    function parseCanvasFont(font) {
        const normalizedFont = String(font ?? "10px sans-serif").trim();
        const sizeMatch = normalizedFont.match(/(\d+(?:\.\d+)?)px/);
        const fontSize = sizeMatch ? Number(sizeMatch[1]) : 10;
        const boldFactor = /\b(600|700|800|900|bold)\b/i.test(normalizedFont) ? 1.08 : 1;
        const monospaceFactor = /\bmonospace\b/i.test(normalizedFont) ? 0.96 : 1;

        return {
            fontSize,
            widthFactor: boldFactor * monospaceFactor,
            normalizedFont,
        };
    }

    function estimateCanvasTextWidth(text, font) {
        const { fontSize, widthFactor } = parseCanvasFont(font);
        const source = String(text ?? "");
        let width = 0;

        for (const character of source) {
            if (character === " ") {
                width += fontSize * 0.33;
                continue;
            }
            if (/[ilI1|]/.test(character)) {
                width += fontSize * 0.34;
                continue;
            }
            if (/[mwMW@#%&]/.test(character)) {
                width += fontSize * 0.92;
                continue;
            }
            if (/[0-9]/.test(character)) {
                width += fontSize * 0.56;
                continue;
            }
            if (character.charCodeAt(0) > 127) {
                width += fontSize * 0.88;
                continue;
            }
            width += fontSize * 0.61;
        }

        return Math.max(0, Number((width * widthFactor).toFixed(3)));
    }

    function createCanvasBitmapState(canvas) {
        const width = normalizeCanvasDimension(canvas?.width, 300);
        const height = normalizeCanvasDimension(canvas?.height, 150);
        let seed = 2166136261 >>> 0;
        seed = hashCanvasNumbers(seed, width, height);

        return {
            width,
            height,
            revision: 0,
            seed,
            path: [],
            operations: [],
        };
    }

    function ensureCanvasBitmapState(canvas) {
        if (!canvas.__canvasBitmapState) {
            canvas.__canvasBitmapState = createCanvasBitmapState(canvas);
        }

        const state = canvas.__canvasBitmapState;
        const width = normalizeCanvasDimension(canvas.width, state.width);
        const height = normalizeCanvasDimension(canvas.height, state.height);

        if (state.width !== width || state.height !== height) {
            canvas.__canvasBitmapState = createCanvasBitmapState(canvas);
            return canvas.__canvasBitmapState;
        }

        return state;
    }

    function appendCanvasOperation(canvas, type, details = {}) {
        const state = ensureCanvasBitmapState(canvas);
        const summary = {
            type: String(type),
            ...details,
        };

        state.revision += 1;
        state.seed = hashCanvasValue(state.seed, summary);
        state.seed = finalizeCanvasSeed(state.seed ^ state.revision);
        state.operations.push(summary);
        if (state.operations.length > 64) {
            state.operations.shift();
        }

        return state;
    }

    function setCanvasPath(canvas, type, details = {}) {
        const state = ensureCanvasBitmapState(canvas);
        state.path.push({
            type: String(type),
            ...details,
        });
        if (state.path.length > 32) {
            state.path.shift();
        }
        return state;
    }

    function clearCanvasPath(canvas) {
        const state = ensureCanvasBitmapState(canvas);
        state.path = [];
        return state;
    }

    function sampleCanvasPixelSeed(state, x, y) {
        let seed = state.seed >>> 0;
        seed = hashCanvasNumbers(seed, x + 1, y + 1, state.width, state.height, state.revision);
        return finalizeCanvasSeed(seed);
    }

    function createCanvasImageData(canvas, x = 0, y = 0, width = 1, height = 1) {
        const state = ensureCanvasBitmapState(canvas);
        const normalizedWidth = Math.max(0, normalizeCanvasInteger(width, 1));
        const normalizedHeight = Math.max(0, normalizeCanvasInteger(height, 1));
        const bytes = new Uint8ClampedArray(normalizedWidth * normalizedHeight * 4);
        let offset = 0;

        for (let row = 0; row < normalizedHeight; row += 1) {
            for (let column = 0; column < normalizedWidth; column += 1) {
                const seed = sampleCanvasPixelSeed(
                    state,
                    normalizeCanvasInteger(x, 0) + column,
                    normalizeCanvasInteger(y, 0) + row,
                );
                bytes[offset] = seed & 0xff;
                bytes[offset + 1] = (seed >>> 8) & 0xff;
                bytes[offset + 2] = (seed >>> 16) & 0xff;
                bytes[offset + 3] = 255;
                offset += 4;
            }
        }

        return {
            data: bytes,
            width: normalizedWidth,
            height: normalizedHeight,
        };
    }

    function normalizeCanvasMimeType(type) {
        const normalized = String(type ?? "image/png").trim().toLowerCase();
        if (normalized === "image/jpeg" || normalized === "image/webp") {
            return normalized;
        }
        return "image/png";
    }

    function canvasHeaderBytes(mimeType) {
        if (mimeType === "image/jpeg") {
            return [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46];
        }
        if (mimeType === "image/webp") {
            return [0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50];
        }
        return [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    }

    function buildCanvasPayloadBytes(canvas, mimeType) {
        const state = ensureCanvasBitmapState(canvas);
        const operationWeight = state.operations.reduce((total, operation) => {
            return total + JSON.stringify(operation).length;
        }, 0);
        const payloadLength = Math.max(
            4096,
            Math.min(
                16384,
                Math.round((state.width * state.height) / 8) +
                    state.revision * 384 +
                    operationWeight * 8,
            ),
        );
        const bytes = new Uint8Array(payloadLength);
        const header = canvasHeaderBytes(mimeType);
        bytes.set(header.slice(0, Math.min(header.length, bytes.length)), 0);
        let rollingSeed = finalizeCanvasSeed(
            hashCanvasNumbers(state.seed, payloadLength, state.revision, state.width, state.height),
        );

        for (let index = header.length; index < bytes.length; index += 1) {
            const x = index % state.width;
            const y = Math.trunc(index / Math.max(1, state.width)) % state.height;
            const pixelSeed = sampleCanvasPixelSeed(state, x, y);
            rollingSeed = finalizeCanvasSeed(
                hashCanvasNumbers(
                    rollingSeed ^ pixelSeed,
                    index + 1,
                    x + 1,
                    y + 1,
                    state.operations.length,
                ),
            );
            bytes[index] = (rollingSeed ^ (pixelSeed >>> ((index % 4) * 8))) & 0xff;
        }

        return bytes;
    }

    function serializeCanvasDataUrl(canvas, type = "image/png") {
        const mimeType = normalizeCanvasMimeType(type);
        const payload = buildCanvasPayloadBytes(canvas, mimeType);
        return `data:${mimeType};base64,${encodeBase64(payload)}`;
    }

    function createCanvasContext(kind, canvas = null) {
        const normalizedKind = String(kind ?? "").toLowerCase();
        if (normalizedKind !== "2d") {
            return null;
        }

        const context = {
            kind: normalizedKind,
            canvas,
            fillStyle: "#000000",
            strokeStyle: "#000000",
            font: "10px sans-serif",
            textBaseline: "alphabetic",
            globalCompositeOperation: "source-over",
            fillRect(x = 0, y = 0, width = 0, height = 0) {
                appendCanvasOperation(this.canvas, "fillRect", {
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    width: normalizeCanvasInteger(width, 0),
                    height: normalizeCanvasInteger(height, 0),
                    fillStyle: this.fillStyle,
                    composite: this.globalCompositeOperation,
                });
            },
            clearRect(x = 0, y = 0, width = 0, height = 0) {
                appendCanvasOperation(this.canvas, "clearRect", {
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    width: normalizeCanvasInteger(width, 0),
                    height: normalizeCanvasInteger(height, 0),
                });
            },
            beginPath() {
                clearCanvasPath(this.canvas);
            },
            rect(x = 0, y = 0, width = 0, height = 0) {
                setCanvasPath(this.canvas, "rect", {
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    width: normalizeCanvasInteger(width, 0),
                    height: normalizeCanvasInteger(height, 0),
                });
            },
            arc(x = 0, y = 0, radius = 0, startAngle = 0, endAngle = 0, counterclockwise = false) {
                setCanvasPath(this.canvas, "arc", {
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    radius: normalizeCanvasInteger(radius, 0),
                    startAngle: Number(startAngle) || 0,
                    endAngle: Number(endAngle) || 0,
                    counterclockwise: Boolean(counterclockwise),
                });
            },
            closePath() {
                setCanvasPath(this.canvas, "closePath");
            },
            fill() {
                const state = ensureCanvasBitmapState(this.canvas);
                appendCanvasOperation(this.canvas, "fill", {
                    fillStyle: this.fillStyle,
                    composite: this.globalCompositeOperation,
                    path: state.path.slice(),
                });
            },
            fillText(text = "", x = 0, y = 0, maxWidth = undefined) {
                appendCanvasOperation(this.canvas, "fillText", {
                    text: String(text),
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    maxWidth: maxWidth === undefined ? null : normalizeCanvasInteger(maxWidth, 0),
                    fillStyle: this.fillStyle,
                    font: this.font,
                    textBaseline: this.textBaseline,
                });
            },
            strokeText(text = "", x = 0, y = 0, maxWidth = undefined) {
                appendCanvasOperation(this.canvas, "strokeText", {
                    text: String(text),
                    x: normalizeCanvasInteger(x, 0),
                    y: normalizeCanvasInteger(y, 0),
                    maxWidth: maxWidth === undefined ? null : normalizeCanvasInteger(maxWidth, 0),
                    strokeStyle: this.strokeStyle,
                    font: this.font,
                    textBaseline: this.textBaseline,
                });
            },
            drawImage(image, ...args) {
                appendCanvasOperation(this.canvas, "drawImage", {
                    sourceWidth: normalizeCanvasInteger(image?.width, 0),
                    sourceHeight: normalizeCanvasInteger(image?.height, 0),
                    args: args.map((value) => normalizeCanvasInteger(value, 0)),
                });
            },
            isPointInPath(x = 0, y = 0) {
                const state = ensureCanvasBitmapState(this.canvas);
                return Boolean(state.path.length) && ((normalizeCanvasInteger(x, 0) + normalizeCanvasInteger(y, 0) + state.revision) % 7 === 0);
            },
            getImageData(x = 0, y = 0, width = 1, height = 1) {
                return createCanvasImageData(this.canvas, x, y, width, height);
            },
            measureText(text = "") {
                const width = estimateCanvasTextWidth(text, this.font);
                const fontSize = parseCanvasFont(this.font).fontSize;
                return new TextMetrics(width, fontSize, String(text).length);
            },
        };

        return context;
    }
