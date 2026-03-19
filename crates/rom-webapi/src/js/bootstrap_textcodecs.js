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
            if (normalizedLabel !== "utf-8" && normalizedLabel !== "utf8") {
                throw new RangeError(`Unsupported encoding: ${label}`);
            }

            defineReadOnly(this, "encoding", "utf-8");
            defineReadOnly(this, "fatal", Boolean(normalizedOptions.fatal));
            defineReadOnly(this, "ignoreBOM", Boolean(normalizedOptions.ignoreBOM));
        }

        decode(input = new Uint8Array()) {
            return decodeUtf8(normalizeTextDecodeInput(input), {
                fatal: this.fatal,
                ignoreBOM: this.ignoreBOM,
            });
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
        let index = 0;

        if (
            !options.ignoreBOM &&
            bytes.length >= 3 &&
            bytes[0] === 0xef &&
            bytes[1] === 0xbb &&
            bytes[2] === 0xbf
        ) {
            index = 3;
        }

        while (index < bytes.length) {
            const first = bytes[index];

            if (first <= 0x7f) {
                codeUnits.push(first);
                index += 1;
                continue;
            }

            const sequence = readUtf8Sequence(bytes, index);
            if (sequence === null) {
                if (fatal) {
                    throw new TypeError("The encoded data was not valid utf-8.");
                }

                codeUnits.push(0xfffd);
                index += 1;
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

        return String.fromCharCode(...codeUnits);
    }

    function readUtf8Sequence(bytes, index) {
        const first = bytes[index];

        if (first >= 0xc2 && first <= 0xdf) {
            const second = bytes[index + 1];
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
            if (
                !isUtf8ContinuationByte(second) ||
                !isUtf8ContinuationByte(third) ||
                (first === 0xe0 && second < 0xa0) ||
                (first === 0xed && second >= 0xa0)
            ) {
                return null;
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
            if (
                !isUtf8ContinuationByte(second) ||
                !isUtf8ContinuationByte(third) ||
                !isUtf8ContinuationByte(fourth) ||
                (first === 0xf0 && second < 0x90) ||
                (first === 0xf4 && second >= 0x90)
            ) {
                return null;
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
