    const textEncoderFactory = class TextEncoder {
        constructor() {
            defineReadOnly(this, "encoding", "utf-8");
        }

        encode(input = "") {
            return Uint8Array.from(encodeUtf8(String(input)));
        }

        encodeInto(input = "", destination) {
            if (!(destination instanceof Uint8Array)) {
                throw new TypeError("TextEncoder.encodeInto requires a Uint8Array destination.");
            }

            const source = String(input);
            let read = 0;
            let written = 0;

            while (read < source.length && written < destination.length) {
                const codePoint = source.codePointAt(read);
                const charLength = codePoint > 0xffff ? 2 : 1;
                const bytes = encodeUtf8(source.slice(read, read + charLength));
                if (written + bytes.length > destination.length) {
                    break;
                }

                destination.set(bytes, written);
                read += charLength;
                written += bytes.length;
            }

            return { read, written };
        }
    };

    const textDecoderFactory = class TextDecoder {
        constructor(label = "utf-8", options = {}) {
            const normalizedLabel = String(label).trim().toLowerCase();
            const normalizedOptions = options == null ? {} : Object(options);
            if (!isUtf8Label(normalizedLabel)) {
                throw new RangeError(`Unsupported encoding: ${label}`);
            }

            defineReadOnly(this, "encoding", "utf-8");
            defineReadOnly(this, "fatal", Boolean(normalizedOptions.fatal));
            defineReadOnly(this, "ignoreBOM", Boolean(normalizedOptions.ignoreBOM));
            this.__pendingBytes = [];
            this.__bomHandled = false;
        }

        decode(input = new Uint8Array(), options = {}) {
            const normalizedOptions = options == null ? {} : Object(options);
            const bytes = [
                ...this.__pendingBytes,
                ...normalizeTextDecodeInput(input),
            ];
            const result = decodeUtf8(bytes, {
                fatal: this.fatal,
                ignoreBOM: this.ignoreBOM,
                stream: Boolean(normalizedOptions.stream),
                bomHandled: this.__bomHandled,
            });

            this.__pendingBytes = result.pendingBytes;
            this.__bomHandled = result.bomHandled;
            return result.text;
        }
    };

    function encodeUtf8(input) {
        const bytes = [];

        for (const symbol of input) {
            const codePoint = symbol.codePointAt(0);

            if (codePoint <= 0x7f) {
                bytes.push(codePoint);
                continue;
            }

            if (codePoint <= 0x7ff) {
                bytes.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
                continue;
            }

            if (codePoint <= 0xffff) {
                bytes.push(
                    0xe0 | (codePoint >> 12),
                    0x80 | ((codePoint >> 6) & 0x3f),
                    0x80 | (codePoint & 0x3f),
                );
                continue;
            }

            bytes.push(
                0xf0 | (codePoint >> 18),
                0x80 | ((codePoint >> 12) & 0x3f),
                0x80 | ((codePoint >> 6) & 0x3f),
                0x80 | (codePoint & 0x3f),
            );
        }

        return bytes;
    }

    function normalizeTextDecodeInput(input) {
        if (input instanceof Uint8Array) {
            return input;
        }

        if (ArrayBuffer.isView(input)) {
            return new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
        }

        if (input instanceof ArrayBuffer) {
            return new Uint8Array(input);
        }

        return Uint8Array.from(input ?? []);
    }

    function decodeUtf8(input, options = {}) {
        const bytes = Array.from(input);
        const codeUnits = [];
        const fatal = Boolean(options.fatal);
        const stream = Boolean(options.stream);
        let bomHandled = Boolean(options.bomHandled);
        let index = 0;

        if (!bomHandled && !options.ignoreBOM) {
            if (
                bytes.length >= 3 &&
                bytes[0] === 0xef &&
                bytes[1] === 0xbb &&
                bytes[2] === 0xbf
            ) {
                index = 3;
                bomHandled = true;
            } else if (stream && isPotentialUtf8BomPrefix(bytes)) {
                return {
                    text: "",
                    pendingBytes: bytes.slice(),
                    bomHandled,
                };
            } else if (bytes.length > 0) {
                bomHandled = true;
            }
        }

        while (index < bytes.length) {
            const first = bytes[index];

            if (first <= 0x7f) {
                codeUnits.push(first);
                index += 1;
                continue;
            }

            const sequence = readUtf8Sequence(bytes, index);
            if (sequence === null || "invalidLength" in sequence) {
                if (fatal) {
                    throw new TypeError("The encoded data was not valid utf-8.");
                }

                codeUnits.push(0xfffd);
                index += sequence?.invalidLength ?? 1;
                continue;
            }
            if ("truncatedLength" in sequence) {
                if (stream) {
                    return {
                        text: String.fromCharCode(...codeUnits),
                        pendingBytes: bytes.slice(index),
                        bomHandled,
                    };
                }

                if (fatal) {
                    throw new TypeError("The encoded data was not valid utf-8.");
                }

                codeUnits.push(0xfffd);
                index += sequence.truncatedLength;
                continue;
            }

            const { codePoint, length } = sequence;
            if (codePoint <= 0xffff) {
                codeUnits.push(codePoint);
            } else {
                const adjusted = codePoint - 0x10000;
                codeUnits.push(0xd800 + (adjusted >> 10), 0xdc00 + (adjusted & 0x3ff));
            }

            index += length;
        }

        return {
            text: String.fromCharCode(...codeUnits),
            pendingBytes: [],
            bomHandled,
        };
    }

    function readUtf8Sequence(bytes, index) {
        const first = bytes[index];

        if (first >= 0xc2 && first <= 0xdf) {
            const second = bytes[index + 1];
            if (second === undefined) {
                return { truncatedLength: 1 };
            }

            if (!isUtf8ContinuationByte(second)) {
                return null;
            }

            return {
                codePoint: ((first & 0x1f) << 6) | (second & 0x3f),
                length: 2,
            };
        }

        if (first >= 0xe0 && first <= 0xef) {
            const second = bytes[index + 1];
            const third = bytes[index + 2];
            if (second === undefined) {
                return { truncatedLength: 1 };
            }
            if (
                !isUtf8ContinuationByte(second) ||
                (first === 0xe0 && second < 0xa0) ||
                (first === 0xed && second >= 0xa0)
            ) {
                return null;
            }
            if (third === undefined) {
                return { truncatedLength: 2 };
            }
            if (!isUtf8ContinuationByte(third)) {
                return { invalidLength: 2 };
            }

            return {
                codePoint:
                    ((first & 0x0f) << 12) | ((second & 0x3f) << 6) | (third & 0x3f),
                length: 3,
            };
        }

        if (first >= 0xf0 && first <= 0xf4) {
            const second = bytes[index + 1];
            const third = bytes[index + 2];
            const fourth = bytes[index + 3];
            if (second === undefined) {
                return { truncatedLength: 1 };
            }
            if (
                !isUtf8ContinuationByte(second) ||
                (first === 0xf0 && second < 0x90) ||
                (first === 0xf4 && second >= 0x90)
            ) {
                return null;
            }
            if (third === undefined) {
                return { truncatedLength: 2 };
            }
            if (!isUtf8ContinuationByte(third)) {
                return { invalidLength: 2 };
            }
            if (fourth === undefined) {
                return { truncatedLength: 3 };
            }
            if (!isUtf8ContinuationByte(fourth)) {
                return { invalidLength: 3 };
            }

            return {
                codePoint:
                    ((first & 0x07) << 18) |
                    ((second & 0x3f) << 12) |
                    ((third & 0x3f) << 6) |
                    (fourth & 0x3f),
                length: 4,
            };
        }

        return null;
    }

    function isUtf8ContinuationByte(value) {
        return Number.isInteger(value) && (value & 0xc0) === 0x80;
    }

    function isPotentialUtf8BomPrefix(bytes) {
        const bom = [0xef, 0xbb, 0xbf];
        if (bytes.length === 0 || bytes.length >= bom.length) {
            return false;
        }

        return bytes.every((value, index) => value === bom[index]);
    }

    function isUtf8Label(label) {
        switch (label) {
            case "unicode-1-1-utf-8":
            case "unicode11utf8":
            case "unicode20utf8":
            case "utf-8":
            case "utf8":
            case "x-unicode20utf8":
                return true;
            default:
                return false;
        }
    }
