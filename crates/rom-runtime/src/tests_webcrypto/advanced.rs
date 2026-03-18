use crate::{RomRuntime, RuntimeConfig};

#[test]
fn validates_webcrypto_aes_ctr_params() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const key = await crypto.subtle.generateKey(
                    { name: "AES-CTR", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const payload = new Uint8Array(33);

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                const missingCounter = await captureError(() =>
                    crypto.subtle.encrypt(
                        { name: "AES-CTR", length: 64 },
                        key,
                        payload,
                    ),
                );
                const missingLength = await captureError(() =>
                    crypto.subtle.encrypt(
                        { name: "AES-CTR", counter: new Uint8Array(16) },
                        key,
                        payload,
                    ),
                );
                const invalidCounter = await captureError(() =>
                    crypto.subtle.encrypt(
                        { name: "AES-CTR", counter: new Uint8Array(15), length: 64 },
                        key,
                        payload,
                    ),
                );
                const invalidLength = await captureError(() =>
                    crypto.subtle.encrypt(
                        { name: "AES-CTR", counter: new Uint8Array(16), length: 129 },
                        key,
                        payload,
                    ),
                );
                const counterWrap = await captureError(() =>
                    crypto.subtle.encrypt(
                        { name: "AES-CTR", counter: new Uint8Array(16), length: 1 },
                        key,
                        payload,
                    ),
                );

                return {
                    missingCounter,
                    missingLength,
                    invalidCounter,
                    invalidLength,
                    counterWrap,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["missingCounter"]["name"], "TypeError");
    assert_eq!(value["missingLength"]["name"], "TypeError");
    assert_eq!(value["invalidCounter"]["name"], "OperationError");
    assert_eq!(value["invalidLength"]["name"], "OperationError");
    assert_eq!(value["counterWrap"]["name"], "OperationError");
}

#[test]
fn supports_webcrypto_pbkdf2_derivation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const password = encoder.encode("password");
                const salt = encoder.encode("salt");
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    password,
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const bits = await crypto.subtle.deriveBits(
                    { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    256,
                );
                const derivedKey = await crypto.subtle.deriveKey(
                    { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    { name: "AES-GCM", length: 256 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const iv = new Uint8Array(12);
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-GCM", iv },
                    derivedKey,
                    encoder.encode("payload"),
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-GCM", iv },
                    derivedKey,
                    ciphertext,
                );

                return {
                    baseAlgorithm: baseKey.algorithm.name,
                    baseUsages: baseKey.usages.join(","),
                    bitsHex: Array.from(
                        new Uint8Array(bits),
                        (byte) => byte.toString(16).padStart(2, "0"),
                    ).join(""),
                    derivedAlgorithm: derivedKey.algorithm.name,
                    derivedLength: derivedKey.algorithm.length,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["baseAlgorithm"], "PBKDF2");
    assert_eq!(value["baseUsages"], "deriveBits,deriveKey");
    assert_eq!(
        value["bitsHex"],
        "632c2812e46d4604102ba7618e9d6d7d2f8128f6266b4a03264d2a0460b7dcb3"
    );
    assert_eq!(value["derivedAlgorithm"], "AES-GCM");
    assert_eq!(value["derivedLength"], 256);
    assert_eq!(value["plaintext"], "payload");
}

#[test]
fn supports_webcrypto_wrap_and_unwrap_key() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(12);
                const wrappingKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 256 },
                    true,
                    ["encrypt", "decrypt", "wrapKey", "unwrapKey"],
                );
                const sourceKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("secret"),
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const wrapped = await crypto.subtle.wrapKey(
                    "raw",
                    sourceKey,
                    wrappingKey,
                    { name: "AES-GCM", iv },
                );
                const unwrapped = await crypto.subtle.unwrapKey(
                    "raw",
                    wrapped,
                    wrappingKey,
                    { name: "AES-GCM", iv },
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const signature = await crypto.subtle.sign(
                    "HMAC",
                    unwrapped,
                    encoder.encode("payload"),
                );
                const verified = await crypto.subtle.verify(
                    "HMAC",
                    unwrapped,
                    signature,
                    encoder.encode("payload"),
                );

                return {
                    wrappedLength: new Uint8Array(wrapped).length,
                    unwrappedAlgorithm: unwrapped.algorithm.name,
                    unwrappedHash: unwrapped.algorithm.hash.name,
                    verified,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["wrappedLength"], 22);
    assert_eq!(value["unwrappedAlgorithm"], "HMAC");
    assert_eq!(value["unwrappedHash"], "SHA-256");
    assert_eq!(value["verified"], true);
}

#[test]
fn validates_method_specific_webcrypto_key_usages() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(12);
                const wrappingKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 256 },
                    true,
                    ["wrapKey", "unwrapKey"],
                );
                const sourceKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("secret"),
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const wrapped = await crypto.subtle.wrapKey(
                    "raw",
                    sourceKey,
                    wrappingKey,
                    { name: "AES-GCM", iv },
                );
                const unwrapped = await crypto.subtle.unwrapKey(
                    "raw",
                    wrapped,
                    wrappingKey,
                    { name: "AES-GCM", iv },
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );

                let encryptError = "";
                try {
                    await crypto.subtle.encrypt(
                        { name: "AES-GCM", iv },
                        wrappingKey,
                        encoder.encode("payload"),
                    );
                } catch (error) {
                    encryptError = String(error.name);
                }

                let decryptError = "";
                try {
                    await crypto.subtle.decrypt(
                        { name: "AES-GCM", iv },
                        wrappingKey,
                        wrapped,
                    );
                } catch (error) {
                    decryptError = String(error.name);
                }

                return {
                    wrappingUsages: wrappingKey.usages.join(","),
                    wrappedLength: new Uint8Array(wrapped).length,
                    unwrappedAlgorithm: unwrapped.algorithm.name,
                    encryptError,
                    decryptError,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["wrappingUsages"], "wrapKey,unwrapKey");
    assert_eq!(value["wrappedLength"], 22);
    assert_eq!(value["unwrappedAlgorithm"], "HMAC");
    assert_eq!(value["encryptError"], "InvalidAccessError");
    assert_eq!(value["decryptError"], "InvalidAccessError");
}

#[test]
fn supports_webcrypto_hkdf_derivation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("input key"),
                    "HKDF",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const salt = encoder.encode("salt value");
                const info = encoder.encode("context");
                const bits = await crypto.subtle.deriveBits(
                    { name: "HKDF", hash: "SHA-256", salt, info },
                    baseKey,
                    256,
                );
                const derivedKey = await crypto.subtle.deriveKey(
                    { name: "HKDF", hash: "SHA-256", salt, info },
                    baseKey,
                    { name: "HMAC", hash: "SHA-256", length: 256 },
                    true,
                    ["sign", "verify"],
                );
                const signature = await crypto.subtle.sign(
                    "HMAC",
                    derivedKey,
                    encoder.encode("payload"),
                );
                const verified = await crypto.subtle.verify(
                    "HMAC",
                    derivedKey,
                    signature,
                    encoder.encode("payload"),
                );

                return {
                    baseAlgorithm: baseKey.algorithm.name,
                    baseUsages: baseKey.usages.join(","),
                    bitsHex: Array.from(
                        new Uint8Array(bits),
                        (byte) => byte.toString(16).padStart(2, "0"),
                    ).join(""),
                    derivedAlgorithm: derivedKey.algorithm.name,
                    derivedHash: derivedKey.algorithm.hash.name,
                    derivedLength: derivedKey.algorithm.length,
                    derivedUsages: derivedKey.usages.join(","),
                    verified,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["baseAlgorithm"], "HKDF");
    assert_eq!(value["baseUsages"], "deriveBits,deriveKey");
    assert_eq!(
        value["bitsHex"],
        "41b5586358525875c07164667c11acf1e71439386d1eb03c894a8af9fdfd0d31"
    );
    assert_eq!(value["derivedAlgorithm"], "HMAC");
    assert_eq!(value["derivedHash"], "SHA-256");
    assert_eq!(value["derivedLength"], 256);
    assert_eq!(value["derivedUsages"], "sign,verify");
    assert_eq!(value["verified"], true);
}

#[test]
fn supports_webcrypto_aes_kw_wrap_and_derivation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const password = encoder.encode("password");
                const salt = encoder.encode("salt");
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    password,
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const wrappingKey = await crypto.subtle.deriveKey(
                    { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    { name: "AES-KW", length: 192 },
                    true,
                    ["wrapKey", "unwrapKey"],
                );
                const sourceKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const wrapped = await crypto.subtle.wrapKey(
                    "raw",
                    sourceKey,
                    wrappingKey,
                    "AES-KW",
                );
                const unwrapped = await crypto.subtle.unwrapKey(
                    "raw",
                    wrapped,
                    wrappingKey,
                    "AES-KW",
                    { name: "AES-GCM" },
                    true,
                    ["encrypt", "decrypt"],
                );
                const jwk = await crypto.subtle.exportKey("jwk", wrappingKey);
                const imported = await crypto.subtle.importKey(
                    "jwk",
                    jwk,
                    { name: "AES-KW" },
                    true,
                    ["wrapKey", "unwrapKey"],
                );
                const iv = new Uint8Array(12);
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-GCM", iv },
                    unwrapped,
                    encoder.encode("payload"),
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-GCM", iv },
                    unwrapped,
                    ciphertext,
                );

                return {
                    baseAlgorithm: baseKey.algorithm.name,
                    wrappingAlgorithm: wrappingKey.algorithm.name,
                    wrappingLength: wrappingKey.algorithm.length,
                    wrappingUsages: wrappingKey.usages.join(","),
                    wrappedLength: new Uint8Array(wrapped).length,
                    unwrappedAlgorithm: unwrapped.algorithm.name,
                    importedAlgorithm: imported.algorithm.name,
                    importedLength: imported.algorithm.length,
                    jwkAlg: jwk.alg,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["baseAlgorithm"], "PBKDF2");
    assert_eq!(value["wrappingAlgorithm"], "AES-KW");
    assert_eq!(value["wrappingLength"], 192);
    assert_eq!(value["wrappingUsages"], "wrapKey,unwrapKey");
    assert_eq!(value["wrappedLength"], 24);
    assert_eq!(value["unwrappedAlgorithm"], "AES-GCM");
    assert_eq!(value["importedAlgorithm"], "AES-KW");
    assert_eq!(value["importedLength"], 192);
    assert_eq!(value["jwkAlg"], "A192KW");
    assert_eq!(value["plaintext"], "payload");
}
