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
            const normalizedAlgorithm = normalizeAlgorithmObject(algorithm);
            const usages = normalizeCryptoKeyUsages(normalizedAlgorithm, keyUsages);
            validateGenerateKeyAlgorithm(normalizedAlgorithm);
            const response = JSON.parse(
                g.__rom_subtle_generate_key(
                    JSON.stringify({
                        algorithm: serializeNormalizedAlgorithmDescriptor(normalizedAlgorithm),
                        extractable: Boolean(extractable),
                        usages,
                    }),
                ),
            );

            return new CryptoKey(response);
        }

        async importKey(format, keyData, algorithm, extractable, keyUsages) {
            const normalizedFormat = normalizeCryptoKeyFormat(format);
            const normalizedAlgorithm = normalizeAlgorithmObject(algorithm);
            const usages = normalizeCryptoKeyUsages(normalizedAlgorithm, keyUsages);
            validateImportKeyData(normalizedFormat, keyData, normalizedAlgorithm, Boolean(extractable), usages);
            const response = JSON.parse(
                g.__rom_subtle_import_key(
                    JSON.stringify({
                        format: normalizedFormat,
                        key_data: serializeKeyData(normalizedFormat, keyData),
                        algorithm: serializeNormalizedAlgorithmDescriptor(normalizedAlgorithm),
                        extractable: Boolean(extractable),
                        usages,
                    }),
                ),
            );

            return new CryptoKey(response);
        }

        async exportKey(format, key) {
            assertCryptoKey(key);
            assertExtractableCryptoKey(key);
            const normalizedFormat = normalizeCryptoKeyFormat(format);
            validateExportKeyFormat(normalizedFormat, key);
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
            assertCryptoKeyUsage(key, "sign");
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
            assertCryptoKeyUsage(key, "verify");
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
            assertCryptoKeyUsage(key, "encrypt");
            const bytes = toByteArray(data);
            const response = JSON.parse(
                g.__rom_subtle_encrypt(
                    JSON.stringify({
                        algorithm: serializeDataOperationAlgorithm(algorithm, bytes.length),
                        key_id: key.__id,
                        data: bytes,
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async decrypt(algorithm, key, data) {
            assertCryptoKey(key);
            assertCryptoKeyUsage(key, "decrypt");
            const bytes = toByteArray(data);
            const response = JSON.parse(
                g.__rom_subtle_decrypt(
                    JSON.stringify({
                        algorithm: serializeDataOperationAlgorithm(algorithm, bytes.length),
                        key_id: key.__id,
                        data: bytes,
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async deriveBits(algorithm, baseKey, length) {
            assertCryptoKey(baseKey);
            assertCryptoKeyUsage(baseKey, "deriveBits");
            const lengthBits = normalizeDeriveBitsLength(length);
            const response = JSON.parse(
                g.__rom_subtle_derive_bits(
                    JSON.stringify({
                        algorithm: serializeDeriveOperationAlgorithm(algorithm),
                        key_id: baseKey.__id,
                        length: lengthBits,
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
        }

        async deriveKey(algorithm, baseKey, derivedKeyAlgorithm, extractable, keyUsages) {
            assertCryptoKey(baseKey);
            assertCryptoKeyUsage(baseKey, "deriveKey");
            const normalizedAlgorithm = normalizeAlgorithmObject(derivedKeyAlgorithm);
            const lengthBits = resolveDerivedKeyLengthBits(normalizedAlgorithm);
            const response = JSON.parse(
                g.__rom_subtle_derive_bits(
                    JSON.stringify({
                        algorithm: serializeDeriveOperationAlgorithm(algorithm),
                        key_id: baseKey.__id,
                        length: Number(lengthBits),
                    }),
                ),
            );
            const bits = toArrayBuffer(response.bytes ?? []);

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
            assertCryptoKey(wrappingKey);
            assertCryptoKeyUsage(wrappingKey, "wrapKey");
            assertExtractableCryptoKey(key);
            const normalizedFormat = normalizeCryptoKeyFormat(format);
            validateExportKeyFormat(normalizedFormat, key);
            const exported = await this.exportKey(normalizedFormat, key);
            const payload =
                normalizedFormat === "jwk"
                    ? new TextEncoder().encode(JSON.stringify(exported))
                    : exported;
            const payloadBytes = toByteArray(payload);

            const response = JSON.parse(
                g.__rom_subtle_encrypt(
                    JSON.stringify({
                        algorithm: serializeDataOperationAlgorithm(
                            wrapAlgorithm,
                            payloadBytes.length,
                        ),
                        key_id: wrappingKey.__id,
                        data: payloadBytes,
                    }),
                ),
            );

            return toArrayBuffer(response.bytes ?? []);
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
            assertCryptoKey(unwrappingKey);
            assertCryptoKeyUsage(unwrappingKey, "unwrapKey");
            const normalizedFormat = String(format);
            const wrappedBytes = toByteArray(wrappedKey);
            const response = JSON.parse(
                g.__rom_subtle_decrypt(
                    JSON.stringify({
                        algorithm: serializeDataOperationAlgorithm(
                            unwrapAlgorithm,
                            wrappedBytes.length,
                        ),
                        key_id: unwrappingKey.__id,
                        data: wrappedBytes,
                    }),
                ),
            );
            const decrypted = toArrayBuffer(response.bytes ?? []);
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

    function assertCryptoKeyUsage(key, usage) {
        if (key.usages.includes(usage)) {
            return;
        }
        throw createCryptoDomException("InvalidAccessError", `The key does not support ${usage}.`);
    }

    function assertExtractableCryptoKey(key) {
        if (!key.extractable) throw createCryptoDomException("InvalidAccessError", "The key is not extractable.");
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
        return serializeNormalizedAlgorithmDescriptor(normalizeAlgorithmObject(algorithm));
    }

    function serializeNormalizedAlgorithmDescriptor(source) {
        return {
            name: source.name,
            hash: source.hash === null ? null : normalizeHashName(source.hash),
            length: source.length === undefined ? null : Number(source.length),
            counter: toOptionalByteArray(source.counter),
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
            counter: algorithm.counter,
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

    function normalizeCryptoKeyFormat(format) {
        const normalized = String(format);
        if (normalized === "raw" || normalized === "jwk") {
            return normalized;
        }
        throw createCryptoDomException("NotSupportedError", `Unsupported key format: ${normalized}`);
    }

    function validateExportKeyFormat(format, key) {
        const algorithmName = String(key.algorithm?.name ?? "").toUpperCase();
        if (
            format === "jwk" &&
            (algorithmName === "PBKDF2" || algorithmName === "HKDF")
        ) {
            throw createCryptoDomException("NotSupportedError", `Unsupported key export format: ${format}`);
        }
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
        return value === undefined || value === null ? null : toByteArray(value);
    }

    function createCryptoDomException(name, message) {
        const error = new Error(message);
        error.name = name;
        return error;
    }
