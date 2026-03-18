    class CryptoKey {
        constructor(descriptor) {
            this.type = String(descriptor.type ?? "");
            this.extractable = Boolean(descriptor.extractable);
            this.algorithm = Object.freeze(normalizeCryptoKeyAlgorithm(descriptor.algorithm ?? {}));
            this.usages = Object.freeze(Array.from(descriptor.usages ?? [], String));
            defineReadOnly(this, "__id", String(descriptor.id ?? ""));
        }
    }

    class SubtleCrypto {
        async digest(algorithm, data) {
            const response = JSON.parse(
                g.__rom_subtle_digest(
                    JSON.stringify({
                        algorithm: normalizeHashName(algorithm),
                        data: toByteArray(data),
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async generateKey(algorithm, extractable, keyUsages) {
            const response = JSON.parse(
                g.__rom_subtle_generate_key(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        extractable: Boolean(extractable),
                        usages: Array.from(keyUsages ?? [], String),
                    }),
                ),
            );

            return new CryptoKey(response);
        }

        async importKey(format, keyData, algorithm, extractable, keyUsages) {
            const normalizedFormat = String(format);
            const response = JSON.parse(
                g.__rom_subtle_import_key(
                    JSON.stringify({
                        format: normalizedFormat,
                        key_data: serializeKeyData(normalizedFormat, keyData),
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        extractable: Boolean(extractable),
                        usages: Array.from(keyUsages ?? [], String),
                    }),
                ),
            );

            return new CryptoKey(response);
        }

        async exportKey(format, key) {
            assertCryptoKey(key);
            const normalizedFormat = String(format);
            const response = JSON.parse(
                g.__rom_subtle_export_key(
                    JSON.stringify({
                        format: normalizedFormat,
                        key_id: key.__id,
                    }),
                ),
            );

            if (normalizedFormat === "raw") {
                return toArrayBuffer(response.bytes ?? []);
            }

            return response;
        }

        async sign(algorithm, key, data) {
            assertCryptoKey(key);
            const response = JSON.parse(
                g.__rom_subtle_sign(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        key_id: key.__id,
                        data: toByteArray(data),
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async verify(algorithm, key, signature, data) {
            assertCryptoKey(key);
            const response = JSON.parse(
                g.__rom_subtle_verify(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        key_id: key.__id,
                        signature: toByteArray(signature),
                        data: toByteArray(data),
                    }),
                ),
            );

            return Boolean(response.verified);
        }

        async encrypt(algorithm, key, data) {
            assertCryptoKey(key);
            const response = JSON.parse(
                g.__rom_subtle_encrypt(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        key_id: key.__id,
                        data: toByteArray(data),
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async decrypt(algorithm, key, data) {
            assertCryptoKey(key);
            const response = JSON.parse(
                g.__rom_subtle_decrypt(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        key_id: key.__id,
                        data: toByteArray(data),
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async deriveBits(algorithm, baseKey, length) {
            assertCryptoKey(baseKey);
            const response = JSON.parse(
                g.__rom_subtle_derive_bits(
                    JSON.stringify({
                        algorithm: serializeAlgorithmDescriptor(algorithm),
                        key_id: baseKey.__id,
                        length: Number(length),
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async deriveKey(algorithm, baseKey, derivedKeyAlgorithm, extractable, keyUsages) {
            const normalizedAlgorithm = normalizeAlgorithmObject(derivedKeyAlgorithm);
            const lengthBits = getDerivedKeyLengthBits(normalizedAlgorithm);
            const bits = await this.deriveBits(algorithm, baseKey, lengthBits);

            return this.importKey(
                "raw",
                bits,
                normalizedAlgorithm,
                extractable,
                keyUsages,
            );
        }

        async wrapKey(format, key, wrappingKey, wrapAlgorithm) {
            assertCryptoKey(key);
            const normalizedFormat = String(format);
            const exported = await this.exportKey(normalizedFormat, key);
            const payload =
                normalizedFormat === "jwk"
                    ? new TextEncoder().encode(JSON.stringify(exported))
                    : exported;

            return this.encrypt(wrapAlgorithm, wrappingKey, payload);
        }

        async unwrapKey(
            format,
            wrappedKey,
            unwrappingKey,
            unwrapAlgorithm,
            unwrappedKeyAlgorithm,
            extractable,
            keyUsages,
        ) {
            const normalizedFormat = String(format);
            const decrypted = await this.decrypt(unwrapAlgorithm, unwrappingKey, wrappedKey);
            const keyData =
                normalizedFormat === "jwk"
                    ? JSON.parse(new TextDecoder().decode(new Uint8Array(decrypted)))
                    : decrypted;

            return this.importKey(
                normalizedFormat,
                keyData,
                unwrappedKeyAlgorithm,
                extractable,
                keyUsages,
            );
        }
    }

    function createCrypto() {
        return {
            getRandomValues(target) {
                const typedArray = assertIntegerTypedArray(target);
                if (typedArray.byteLength > 65536) {
                    throw new Error("QuotaExceededError");
                }

                const bytes = randomBytes(typedArray.byteLength);
                new Uint8Array(
                    typedArray.buffer,
                    typedArray.byteOffset,
                    typedArray.byteLength,
                ).set(Uint8Array.from(bytes));
                return target;
            },
            randomUUID() {
                const bytes = Uint8Array.from(randomBytes(16));
                bytes[6] = (bytes[6] & 0x0f) | 0x40;
                bytes[8] = (bytes[8] & 0x3f) | 0x80;
                const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0"));
                return [
                    hex.slice(0, 4).join(""),
                    hex.slice(4, 6).join(""),
                    hex.slice(6, 8).join(""),
                    hex.slice(8, 10).join(""),
                    hex.slice(10, 16).join(""),
                ].join("-");
            },
            subtle: new SubtleCrypto(),
        };
    }

    function randomBytes(length) {
        const response = JSON.parse(g.__rom_random_bytes(Number(length)));
        return response.bytes ?? [];
    }

    function assertCryptoKey(value) {
        if (!(value instanceof CryptoKey)) {
            throw new TypeError("Expected CryptoKey");
        }
    }

    function assertIntegerTypedArray(target) {
        if (
            target instanceof Int8Array ||
            target instanceof Uint8Array ||
            target instanceof Uint8ClampedArray ||
            target instanceof Int16Array ||
            target instanceof Uint16Array ||
            target instanceof Int32Array ||
            target instanceof Uint32Array
        ) {
            return target;
        }

        throw new TypeError("Expected an integer TypedArray");
    }

    function normalizeHashName(algorithm) {
        if (typeof algorithm === "string") {
            return algorithm;
        }

        if (algorithm && typeof algorithm === "object" && algorithm.name !== undefined) {
            return String(algorithm.name);
        }

        throw new TypeError("Invalid algorithm identifier");
    }

    function serializeAlgorithmDescriptor(algorithm) {
        const source = normalizeAlgorithmObject(algorithm);

        return {
            name: source.name,
            hash: source.hash === null ? null : normalizeHashName(source.hash),
            length: source.length === undefined ? null : Number(source.length),
            iv: toOptionalByteArray(source.iv),
            additional_data: toOptionalByteArray(source.additionalData),
            tag_length: source.tagLength === undefined ? null : Number(source.tagLength),
            salt: toOptionalByteArray(source.salt),
            info: toOptionalByteArray(source.info),
            iterations: source.iterations === undefined ? null : Number(source.iterations),
        };
    }

    function normalizeCryptoKeyAlgorithm(algorithm) {
        const source = normalizeAlgorithmObject(algorithm);
        const hashName = source.hash === null ? null : normalizeHashName(source.hash);

        return {
            name: source.name,
            hash: hashName === null ? null : { name: hashName },
            length: source.length === undefined ? null : Number(source.length),
        };
    }

    function normalizeAlgorithmObject(algorithm) {
        if (typeof algorithm === "string") {
            return { name: algorithm, hash: null };
        }

        if (!algorithm || typeof algorithm !== "object") {
            throw new TypeError("Invalid algorithm identifier");
        }

        return {
            name: String(algorithm.name ?? ""),
            hash: algorithm.hash ?? null,
            length: algorithm.length,
            iv: algorithm.iv,
            additionalData: algorithm.additionalData,
            tagLength: algorithm.tagLength,
            salt: algorithm.salt,
            info: algorithm.info,
            iterations: algorithm.iterations,
        };
    }

    function serializeKeyData(format, keyData) {
        if (format === "raw") {
            return toByteArray(keyData);
        }

        if (format === "jwk") {
            return keyData;
        }

        throw new TypeError(`Unsupported key format: ${format}`);
    }

    function toByteArray(value) {
        if (value instanceof ArrayBuffer) {
            return Array.from(new Uint8Array(value));
        }

        if (ArrayBuffer.isView(value)) {
            return Array.from(new Uint8Array(value.buffer, value.byteOffset, value.byteLength));
        }

        if (Array.isArray(value)) {
            return value.map((entry) => Number(entry) & 0xff);
        }

        throw new TypeError("Expected ArrayBuffer or TypedArray input");
    }

    function toArrayBuffer(bytes) {
        const view = Uint8Array.from(bytes);
        return view.buffer.slice(view.byteOffset, view.byteOffset + view.byteLength);
    }

    function toOptionalByteArray(value) {
        if (value === undefined || value === null) {
            return null;
        }

        return toByteArray(value);
    }

    function getDerivedKeyLengthBits(algorithm) {
        const normalizedName = String(algorithm.name ?? "").toUpperCase();
        if (normalizedName === "AES-GCM") {
            if (algorithm.length === undefined) {
                throw new TypeError("Derived AES-GCM key requires algorithm.length");
            }
            return Number(algorithm.length);
        }

        if (normalizedName === "HMAC") {
            if (algorithm.length !== undefined) {
                return Number(algorithm.length);
            }
            return defaultHmacLengthBits(algorithm.hash);
        }

        throw new TypeError(`Unsupported deriveKey target: ${algorithm.name}`);
    }

    function defaultHmacLengthBits(hash) {
        switch (normalizeHashName(hash).toUpperCase()) {
            case "SHA-1":
            case "SHA-256":
                return 512;
            case "SHA-384":
            case "SHA-512":
                return 1024;
            default:
                throw new TypeError(`Unsupported HMAC hash: ${hash}`);
        }
    }
