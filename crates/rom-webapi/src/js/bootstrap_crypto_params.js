function serializeDataOperationAlgorithm(algorithm, dataLength) {
    const source = normalizeAlgorithmObject(algorithm);
    validateDataOperationAlgorithm(source, dataLength);
    return serializeNormalizedAlgorithmDescriptor(source);
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
