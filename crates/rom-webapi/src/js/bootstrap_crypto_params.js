function serializeDataOperationAlgorithm(algorithm, dataLength) {
    const source = normalizeAlgorithmObject(algorithm);
    validateDataOperationAlgorithm(source, dataLength);
    return serializeNormalizedAlgorithmDescriptor(source);
}

function serializeWrapOperationAlgorithm(algorithm, dataLength) {
    const source = normalizeAlgorithmObject(algorithm);
    validateWrapOperationAlgorithm(source, dataLength);
    return serializeNormalizedAlgorithmDescriptor(source);
}

function serializeDeriveOperationAlgorithm(algorithm) {
    const source = normalizeAlgorithmObject(algorithm);
    validateDeriveOperationAlgorithm(source);
    return serializeNormalizedAlgorithmDescriptor(source);
}

function validateGenerateKeyAlgorithm(algorithm) {
    switch (String(algorithm.name ?? "").toUpperCase()) {
        case "HMAC":
            validateHmacKeyLength(algorithm, null, "OperationError");
            requireAlgorithmHash(algorithm, "HMAC");
            break;
        case "AES-CTR":
        case "AES-CBC":
        case "AES-GCM":
        case "AES-KW":
            validateAesKeyLength(String(algorithm.name), algorithm.length, "OperationError");
            break;
    }
}

function resolveDerivedKeyLengthBits(algorithm) {
    const algorithmName = String(algorithm.name ?? "").toUpperCase();
    switch (algorithmName) {
        case "HMAC":
            validateGenerateKeyAlgorithm(algorithm);
            return algorithm.length === undefined
                ? defaultHmacLengthBits(algorithm.hash)
                : Number(algorithm.length);
        case "AES-CTR":
        case "AES-CBC":
        case "AES-GCM":
        case "AES-KW":
            validateGenerateKeyAlgorithm(algorithm);
            return Number(algorithm.length);
        default:
            throw createCryptoDomException("NotSupportedError", `Unsupported deriveKey target: ${algorithm.name}`);
    }
}

function normalizeCryptoKeyUsages(algorithm, keyUsages) {
    const usages = Array.from(keyUsages ?? [], String);
    validateCryptoKeyUsages(algorithm, usages);
    return usages;
}

function validateImportKeyData(format, keyData, algorithm, extractable, usages) {
    if (format === "raw") {
        validateRawSecretImport(algorithm, toByteArray(keyData));
        return;
    }
    if (!keyData || typeof keyData !== "object" || Array.isArray(keyData)) {
        throw new TypeError("JWK keyData must be an object");
    }
    validateSecretJwkImport(algorithm, keyData, extractable, usages);
}

function validateDataOperationAlgorithm(algorithm, dataLength) {
    switch (String(algorithm.name ?? "").toUpperCase()) {
        case "AES-CTR":
            validateAesCtrParams(algorithm, dataLength);
            break;
        case "AES-CBC":
            validateAesCbcParams(algorithm);
            break;
        case "AES-GCM":
            validateAesGcmParams(algorithm);
            break;
        default:
            throw createCryptoDomException(
                "InvalidAccessError",
                `Unsupported data operation algorithm: ${algorithm.name}`,
            );
    }
}

function validateWrapOperationAlgorithm(algorithm, dataLength) {
    switch (String(algorithm.name ?? "").toUpperCase()) {
        case "AES-CTR":
        case "AES-CBC":
        case "AES-GCM":
            validateDataOperationAlgorithm(algorithm, dataLength);
            break;
        case "AES-KW":
            if (dataLength < 16 || dataLength % 8 !== 0) {
                throw createCryptoDomException("OperationError", "AES-KW payload must be 64-bit aligned and at least 128 bits.");
            }
            break;
        default:
            throw createCryptoDomException(
                "NotSupportedError",
                `Unsupported wrap algorithm: ${algorithm.name}`,
            );
    }
}

function validateDeriveOperationAlgorithm(algorithm) {
    switch (String(algorithm.name ?? "").toUpperCase()) {
        case "PBKDF2":
            validatePbkdf2Params(algorithm);
            break;
        case "HKDF":
            validateHkdfParams(algorithm);
            break;
        default:
            throw createCryptoDomException(
                "NotSupportedError",
                `Unsupported derive algorithm: ${algorithm.name}`,
            );
    }
}

function validateCryptoKeyUsages(algorithm, usages) {
    switch (String(algorithm.name ?? "").toUpperCase()) {
        case "HMAC":
            validateSecretKeyUsages("HMAC", usages, ["sign", "verify"]);
            break;
        case "AES-CTR":
        case "AES-CBC":
        case "AES-GCM":
            validateSecretKeyUsages(algorithm.name, usages, ["encrypt", "decrypt", "wrapKey", "unwrapKey"]);
            break;
        case "AES-KW":
            validateSecretKeyUsages("AES-KW", usages, ["wrapKey", "unwrapKey"]);
            break;
        case "PBKDF2":
        case "HKDF":
            validateSecretKeyUsages(algorithm.name, usages, ["deriveBits", "deriveKey"]);
            break;
    }
}

function validateAesCtrParams(algorithm, dataLength) {
    const counter = requireAlgorithmBytes(algorithm, "counter", "AES-CTR");
    const length = requireAlgorithmInteger(algorithm, "length", "AES-CTR");
    if (counter.length !== 16 || length < 1 || length > 128) {
        throwInvalidAlgorithmParams("AES-CTR");
    }

    const blocks = BigInt(Math.ceil(Number(dataLength) / 16));
    if (blocks === 0n) {
        return;
    }

    const counterValue = bytesToBigInt(counter);
    if (length === 128) {
        if ((blocks - 1n) > ((1n << 128n) - 1n - counterValue)) {
            throw createCryptoDomException("OperationError", "AES-CTR counter would wrap.");
        }
        return;
    }

    const space = 1n << BigInt(length);
    if (blocks > (space - (counterValue & (space - 1n)))) {
        throw createCryptoDomException("OperationError", "AES-CTR counter would wrap.");
    }
}

function validateAesCbcParams(algorithm) {
    if (requireAlgorithmBytes(algorithm, "iv", "AES-CBC").length !== 16) {
        throwInvalidAlgorithmParams("AES-CBC");
    }
}

function validateAesGcmParams(algorithm) {
    if (requireAlgorithmBytes(algorithm, "iv", "AES-GCM").length !== 12) {
        throwInvalidAlgorithmParams("AES-GCM");
    }

    if (algorithm.tagLength === undefined) {
        return;
    }

    const tagLength = Number(algorithm.tagLength);
    if (!Number.isInteger(tagLength) || !isSupportedAesGcmTagLength(tagLength)) {
        throwInvalidAlgorithmParams("AES-GCM");
    }
}

function isSupportedAesGcmTagLength(tagLength) {
    return (
        tagLength === 96 ||
        tagLength === 104 ||
        tagLength === 112 ||
        tagLength === 120 ||
        tagLength === 128
    );
}

function validatePbkdf2Params(algorithm) {
    requireAlgorithmBytes(algorithm, "salt", "PBKDF2");
    requireAlgorithmHash(algorithm, "PBKDF2");
    if (requireAlgorithmInteger(algorithm, "iterations", "PBKDF2") <= 0) {
        throwInvalidAlgorithmParams("PBKDF2");
    }
}

function validateHkdfParams(algorithm) {
    requireAlgorithmBytes(algorithm, "salt", "HKDF");
    requireAlgorithmBytes(algorithm, "info", "HKDF");
    requireAlgorithmHash(algorithm, "HKDF");
}

function validateSecretKeyUsages(algorithmName, usages, allowedUsages) {
    if (usages.length === 0) {
        throw createCryptoDomException("SyntaxError", `${algorithmName} keys require at least one usage.`);
    }

    for (const usage of usages) {
        if (!allowedUsages.includes(usage)) {
            throw createCryptoDomException(
                "SyntaxError",
                `Invalid key usage for ${algorithmName}: ${usage}.`,
            );
        }
    }
}

function validateSecretJwkImport(algorithm, jwk, extractable, usages) {
    const algorithmName = String(algorithm.name ?? "").toUpperCase();
    if (algorithmName === "PBKDF2" || algorithmName === "HKDF") {
        throw createCryptoDomException("NotSupportedError", `Unsupported key import format for ${algorithmName}: jwk`);
    }
    if (jwk.kty !== "oct") {
        throw createCryptoDomException("DataError", `Unsupported JWK kty for ${algorithm.name}`);
    }
    if (typeof jwk.k !== "string") {
        throw createCryptoDomException("DataError", "JWK key material must be a string.");
    }

    const secret = decodeBase64Url(jwk.k);
    validateImportedSecretLength(algorithm, secret.length);
    if (jwk.ext === false && extractable) {
        throw createCryptoDomException("DataError", "JWK ext does not allow extractable import.");
    }

    const expectedUse = algorithmName === "HMAC" ? "sig" : "enc";
    if (jwk.use !== undefined && jwk.use !== expectedUse) {
        throw createCryptoDomException("DataError", `JWK use mismatch: expected ${expectedUse}.`);
    }
    if (jwk.key_ops !== undefined) {
        if (!Array.isArray(jwk.key_ops) || jwk.key_ops.some((usage) => typeof usage !== "string")) {
            throw createCryptoDomException("DataError", "JWK key_ops must be an array of strings.");
        }
        if (!usages.every((usage) => jwk.key_ops.includes(usage))) {
            throw createCryptoDomException("DataError", "JWK key_ops do not allow the requested usages.");
        }
    }

    const expectedAlg = expectedJwkAlgorithm(algorithmName, algorithm, secret.length);
    if (jwk.alg !== undefined && jwk.alg !== expectedAlg) {
        throw createCryptoDomException("DataError", `JWK alg mismatch: expected ${expectedAlg}.`);
    }
}

function validateRawSecretImport(algorithm, secret) {
    validateImportedSecretLength(algorithm, secret.length);
}

function validateImportedSecretLength(algorithm, secretLengthBytes) {
    const algorithmName = String(algorithm.name ?? "").toUpperCase();
    const secretLengthBits = secretLengthBytes * 8;

    switch (algorithmName) {
        case "HMAC":
            validateHmacKeyLength(algorithm, secretLengthBits, "DataError");
            requireAlgorithmHash(algorithm, "HMAC");
            break;
        case "AES-CTR":
        case "AES-CBC":
        case "AES-GCM":
        case "AES-KW":
            validateAesImportLength(String(algorithm.name), algorithm.length, secretLengthBits);
            break;
    }
}

function validateHmacKeyLength(algorithm, secretLengthBits, errorName) {
    if (algorithm.length === undefined) {
        return;
    }

    const length = Number(algorithm.length);
    if (!Number.isInteger(length) || length <= 0) {
        throw createCryptoDomException(errorName, "Invalid HMAC key length.");
    }
    if (secretLengthBits !== null && length > secretLengthBits) {
        throw createCryptoDomException("DataError", "HMAC key length exceeds keyData.");
    }
}

function validateAesImportLength(algorithmName, declaredLength, secretLengthBits) {
    validateAesKeyLength(algorithmName, secretLengthBits, "DataError");
    if (declaredLength === undefined) {
        return;
    }

    const normalizedLength = normalizeAesKeyLength(algorithmName, declaredLength, "DataError");
    if (normalizedLength !== secretLengthBits) {
        throw createCryptoDomException("DataError", `${algorithmName} key length does not match keyData.`);
    }
}

function validateAesKeyLength(algorithmName, length, errorName) {
    normalizeAesKeyLength(algorithmName, length, errorName);
}

function normalizeAesKeyLength(algorithmName, length, errorName) {
    if (length === undefined) {
        throw new TypeError(`${algorithmName} requires algorithm.length`);
    }

    const normalizedLength = Number(length);
    if (
        !Number.isInteger(normalizedLength) ||
        (normalizedLength !== 128 && normalizedLength !== 192 && normalizedLength !== 256)
    ) {
        throw createCryptoDomException(errorName, `Invalid ${algorithmName} key length.`);
    }

    return normalizedLength;
}

function defaultHmacLengthBits(hash) {
    switch (normalizeHashName(hash).toUpperCase()) {
        case "SHA-1":
            return 160;
        case "SHA-256":
            return 256;
        case "SHA-384":
            return 384;
        case "SHA-512":
            return 512;
        default:
            throw new TypeError(`Unsupported HMAC hash: ${hash}`);
    }
}

function expectedJwkAlgorithm(algorithmName, algorithm, secretLength) {
    switch (algorithmName) {
        case "HMAC":
            return expectedHmacJwkAlgorithm(algorithm);
        case "AES-CTR":
        case "AES-CBC":
        case "AES-KW":
            return expectedAesJwkAlgorithm(algorithmName, secretLength);
        case "AES-GCM":
            if (secretLength === 16) return "A128GCM";
            if (secretLength === 24) return "A192GCM";
            if (secretLength === 32) return "A256GCM";
            return invalidJwkSecretLength(algorithmName, secretLength);
        default:
            throw createCryptoDomException("NotSupportedError", `Unsupported key import format for ${algorithmName}: jwk`);
    }
}

function expectedHmacJwkAlgorithm(algorithm) {
    switch (requireAlgorithmHash(algorithm, "HMAC").toUpperCase()) {
        case "SHA-1":
            return "HS1";
        case "SHA-256":
            return "HS256";
        case "SHA-384":
            return "HS384";
        case "SHA-512":
            return "HS512";
        default:
            throw createCryptoDomException("DataError", "Unsupported HMAC JWK hash.");
    }
}

function expectedAesJwkAlgorithm(algorithmName, secretLength) {
    switch (`${algorithmName}:${secretLength}`) {
        case "AES-CTR:16":
            return "A128CTR";
        case "AES-CTR:24":
            return "A192CTR";
        case "AES-CTR:32":
            return "A256CTR";
        case "AES-CBC:16":
            return "A128CBC";
        case "AES-CBC:24":
            return "A192CBC";
        case "AES-CBC:32":
            return "A256CBC";
        case "AES-KW:16":
            return "A128KW";
        case "AES-KW:24":
            return "A192KW";
        case "AES-KW:32":
            return "A256KW";
        default:
            return invalidJwkSecretLength(algorithmName, secretLength);
    }
}

function invalidJwkSecretLength(algorithmName, secretLength) {
    throw createCryptoDomException("DataError", `Unsupported ${algorithmName} raw key length: ${secretLength * 8} bits`);
}

function decodeBase64Url(value) {
    if (!/^[A-Za-z0-9_-]*$/.test(value) || value.length % 4 === 1) {
        throw createCryptoDomException("DataError", "Invalid JWK key material.");
    }

    const input = `${value.replace(/-/g, "+").replace(/_/g, "/")}${"=".repeat((4 - (value.length % 4 || 4)) % 4)}`;
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const bytes = [];

    for (let index = 0; index < input.length; index += 4) {
        const chars = input.slice(index, index + 4);
        const codes = Array.from(chars, (char) => (char === "=" ? 0 : alphabet.indexOf(char)));
        if (codes.some((code) => code < 0)) {
            throw createCryptoDomException("DataError", "Invalid JWK key material.");
        }
        const triple = (codes[0] << 18) | (codes[1] << 12) | (codes[2] << 6) | codes[3];
        bytes.push((triple >> 16) & 0xff);
        if (chars[2] !== "=") bytes.push((triple >> 8) & 0xff);
        if (chars[3] !== "=") bytes.push(triple & 0xff);
    }

    return bytes;
}

function normalizeDeriveBitsLength(length) {
    if (length === null) {
        throw createCryptoDomException(
            "OperationError",
            "deriveBits length must not be null.",
        );
    }
    const normalized = Number(length);
    if (!Number.isInteger(normalized) || normalized % 8 !== 0) {
        throw createCryptoDomException(
            "OperationError",
            "deriveBits length must be a multiple of 8.",
        );
    }
    return normalized;
}

function requireAlgorithmBytes(algorithm, field, algorithmName) {
    if (algorithm[field] === undefined) {
        throw new TypeError(`${algorithmName} requires algorithm.${field}`);
    }

    return toByteArray(algorithm[field]);
}

function requireAlgorithmInteger(algorithm, field, algorithmName) {
    if (algorithm[field] === undefined) {
        throw new TypeError(`${algorithmName} requires algorithm.${field}`);
    }

    const value = Number(algorithm[field]);
    if (!Number.isInteger(value)) {
        throwInvalidAlgorithmParams(algorithmName);
    }
    return value;
}

function requireAlgorithmHash(algorithm, algorithmName) {
    if (algorithm.hash === undefined || algorithm.hash === null) {
        throw new TypeError(`${algorithmName} requires algorithm.hash`);
    }
    return normalizeHashName(algorithm.hash);
}

function throwInvalidAlgorithmParams(algorithmName) {
    throw createCryptoDomException("OperationError", `Invalid ${algorithmName} parameters.`);
}

function bytesToBigInt(bytes) {
    let value = 0n;
    for (const byte of bytes) {
        value = (value << 8n) | BigInt(byte);
    }
    return value;
}
