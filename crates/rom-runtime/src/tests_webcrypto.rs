use crate::{RomRuntime, RuntimeConfig};

#[test]
fn supports_webcrypto_digest_and_randomness() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const digest = await crypto.subtle.digest(
                    "SHA-256",
                    new TextEncoder().encode("abc"),
                );
                const random = new Uint8Array(8);
                crypto.getRandomValues(random);
                const uuid = crypto.randomUUID();

                return {
                    digestHex: Array.from(
                        new Uint8Array(digest),
                        (byte) => byte.toString(16).padStart(2, "0"),
                    ).join(""),
                    randomLength: random.length,
                    randomNonZero: Array.from(random).some((value) => value !== 0),
                    uuid,
                    uuidVersion: uuid.split("-")[2][0],
                    uuidVariant: uuid.split("-")[3][0],
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();
    let uuid = value["uuid"].as_str().unwrap();

    assert_eq!(
        value["digestHex"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(value["randomLength"], 8);
    assert_eq!(value["randomNonZero"], true);
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.chars().nth(8), Some('-'));
    assert_eq!(uuid.chars().nth(13), Some('-'));
    assert_eq!(uuid.chars().nth(18), Some('-'));
    assert_eq!(uuid.chars().nth(23), Some('-'));
    assert_eq!(value["uuidVersion"], "4");
    assert!(matches!(
        value["uuidVariant"].as_str().unwrap(),
        "8" | "9" | "a" | "b"
    ));
}

#[test]
fn supports_webcrypto_hmac_lifecycle() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const payload = encoder.encode("payload");
                const secret = encoder.encode("secret");
                const key = await crypto.subtle.importKey(
                    "raw",
                    secret,
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const signature = await crypto.subtle.sign("HMAC", key, payload);
                const verified = await crypto.subtle.verify("HMAC", key, signature, payload);
                const rejected = await crypto.subtle.verify(
                    "HMAC",
                    key,
                    signature,
                    encoder.encode("tampered"),
                );
                const raw = await crypto.subtle.exportKey("raw", key);
                const jwk = await crypto.subtle.exportKey("jwk", key);
                const jwkKey = await crypto.subtle.importKey(
                    "jwk",
                    jwk,
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const jwkVerified = await crypto.subtle.verify(
                    "HMAC",
                    jwkKey,
                    signature,
                    payload,
                );
                const generated = await crypto.subtle.generateKey(
                    { name: "HMAC", hash: "SHA-512", length: 256 },
                    true,
                    ["sign", "verify"],
                );
                const generatedJwk = await crypto.subtle.exportKey("jwk", generated);

                return {
                    keyType: key.type,
                    keyAlgorithm: key.algorithm.name,
                    keyHash: key.algorithm.hash.name,
                    keyLength: key.algorithm.length,
                    keyUsages: key.usages.join(","),
                    signatureLength: new Uint8Array(signature).length,
                    verified,
                    rejected,
                    rawSecret: new TextDecoder().decode(new Uint8Array(raw)),
                    jwkAlg: jwk.alg,
                    jwkKty: jwk.kty,
                    jwkVerified,
                    generatedAlg: generatedJwk.alg,
                    generatedKeyOps: generatedJwk.key_ops.join(","),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["keyType"], "secret");
    assert_eq!(value["keyAlgorithm"], "HMAC");
    assert_eq!(value["keyHash"], "SHA-256");
    assert_eq!(value["keyLength"], 48);
    assert_eq!(value["keyUsages"], "sign,verify");
    assert_eq!(value["signatureLength"], 32);
    assert_eq!(value["verified"], true);
    assert_eq!(value["rejected"], false);
    assert_eq!(value["rawSecret"], "secret");
    assert_eq!(value["jwkAlg"], "HS256");
    assert_eq!(value["jwkKty"], "oct");
    assert_eq!(value["jwkVerified"], true);
    assert_eq!(value["generatedAlg"], "HS512");
    assert_eq!(value["generatedKeyOps"], "sign,verify");
}

#[test]
fn supports_webcrypto_aes_gcm_lifecycle() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(12);
                const aad = encoder.encode("aad");
                const payload = encoder.encode("payload");
                const key = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 256 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-GCM", iv, additionalData: aad, tagLength: 128 },
                    key,
                    payload,
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-GCM", iv, additionalData: aad, tagLength: 128 },
                    key,
                    ciphertext,
                );
                const raw = await crypto.subtle.exportKey("raw", key);
                const jwk = await crypto.subtle.exportKey("jwk", key);
                const imported = await crypto.subtle.importKey(
                    "raw",
                    raw,
                    { name: "AES-GCM" },
                    true,
                    ["encrypt", "decrypt"],
                );
                const importedPlaintext = await crypto.subtle.decrypt(
                    { name: "AES-GCM", iv, additionalData: aad },
                    imported,
                    ciphertext,
                );

                return {
                    keyType: key.type,
                    keyAlgorithm: key.algorithm.name,
                    keyLength: key.algorithm.length,
                    cipherLength: new Uint8Array(ciphertext).length,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                    importedPlaintext: new TextDecoder().decode(new Uint8Array(importedPlaintext)),
                    rawLength: new Uint8Array(raw).length,
                    jwkAlg: jwk.alg,
                    jwkKty: jwk.kty,
                    keyUsages: key.usages.join(","),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["keyType"], "secret");
    assert_eq!(value["keyAlgorithm"], "AES-GCM");
    assert_eq!(value["keyLength"], 256);
    assert_eq!(value["cipherLength"], 23);
    assert_eq!(value["plaintext"], "payload");
    assert_eq!(value["importedPlaintext"], "payload");
    assert_eq!(value["rawLength"], 32);
    assert_eq!(value["jwkAlg"], "A256GCM");
    assert_eq!(value["jwkKty"], "oct");
    assert_eq!(value["keyUsages"], "encrypt,decrypt");
}

#[test]
fn supports_webcrypto_aes_cbc_lifecycle_and_derivation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(16);
                const payload = encoder.encode("payload");
                const key = await crypto.subtle.generateKey(
                    { name: "AES-CBC", length: 192 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-CBC", iv },
                    key,
                    payload,
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-CBC", iv },
                    key,
                    ciphertext,
                );
                const raw = await crypto.subtle.exportKey("raw", key);
                const jwk = await crypto.subtle.exportKey("jwk", key);
                const imported = await crypto.subtle.importKey(
                    "jwk",
                    jwk,
                    { name: "AES-CBC" },
                    true,
                    ["encrypt", "decrypt"],
                );
                const importedPlaintext = await crypto.subtle.decrypt(
                    { name: "AES-CBC", iv },
                    imported,
                    ciphertext,
                );
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("password"),
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const derivedKey = await crypto.subtle.deriveKey(
                    { name: "PBKDF2", salt: encoder.encode("salt"), iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    { name: "AES-CBC", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const derivedCiphertext = await crypto.subtle.encrypt(
                    { name: "AES-CBC", iv },
                    derivedKey,
                    payload,
                );
                const derivedPlaintext = await crypto.subtle.decrypt(
                    { name: "AES-CBC", iv },
                    derivedKey,
                    derivedCiphertext,
                );

                return {
                    keyType: key.type,
                    keyAlgorithm: key.algorithm.name,
                    keyLength: key.algorithm.length,
                    keyUsages: key.usages.join(","),
                    cipherLength: new Uint8Array(ciphertext).length,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                    importedPlaintext: new TextDecoder().decode(new Uint8Array(importedPlaintext)),
                    rawLength: new Uint8Array(raw).length,
                    jwkAlg: jwk.alg,
                    jwkKty: jwk.kty,
                    derivedAlgorithm: derivedKey.algorithm.name,
                    derivedLength: derivedKey.algorithm.length,
                    derivedPlaintext: new TextDecoder().decode(new Uint8Array(derivedPlaintext)),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["keyType"], "secret");
    assert_eq!(value["keyAlgorithm"], "AES-CBC");
    assert_eq!(value["keyLength"], 192);
    assert_eq!(value["keyUsages"], "encrypt,decrypt");
    assert_eq!(value["cipherLength"], 16);
    assert_eq!(value["plaintext"], "payload");
    assert_eq!(value["importedPlaintext"], "payload");
    assert_eq!(value["rawLength"], 24);
    assert_eq!(value["jwkAlg"], "A192CBC");
    assert_eq!(value["jwkKty"], "oct");
    assert_eq!(value["derivedAlgorithm"], "AES-CBC");
    assert_eq!(value["derivedLength"], 128);
    assert_eq!(value["derivedPlaintext"], "payload");
}

#[test]
fn supports_webcrypto_aes_ctr_lifecycle_and_derivation() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const counter = new Uint8Array(16);
                counter[15] = 1;
                const payload = encoder.encode("payload");
                const key = await crypto.subtle.generateKey(
                    { name: "AES-CTR", length: 256 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-CTR", counter, length: 64 },
                    key,
                    payload,
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-CTR", counter, length: 64 },
                    key,
                    ciphertext,
                );
                const raw = await crypto.subtle.exportKey("raw", key);
                const jwk = await crypto.subtle.exportKey("jwk", key);
                const imported = await crypto.subtle.importKey(
                    "jwk",
                    jwk,
                    { name: "AES-CTR" },
                    true,
                    ["encrypt", "decrypt"],
                );
                const importedPlaintext = await crypto.subtle.decrypt(
                    { name: "AES-CTR", counter, length: 64 },
                    imported,
                    ciphertext,
                );
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("password"),
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const derivedKey = await crypto.subtle.deriveKey(
                    { name: "PBKDF2", salt: encoder.encode("salt"), iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    { name: "AES-CTR", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const derivedCiphertext = await crypto.subtle.encrypt(
                    { name: "AES-CTR", counter, length: 64 },
                    derivedKey,
                    payload,
                );
                const derivedPlaintext = await crypto.subtle.decrypt(
                    { name: "AES-CTR", counter, length: 64 },
                    derivedKey,
                    derivedCiphertext,
                );

                return {
                    keyType: key.type,
                    keyAlgorithm: key.algorithm.name,
                    keyLength: key.algorithm.length,
                    keyUsages: key.usages.join(","),
                    cipherLength: new Uint8Array(ciphertext).length,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                    importedPlaintext: new TextDecoder().decode(new Uint8Array(importedPlaintext)),
                    rawLength: new Uint8Array(raw).length,
                    jwkAlg: jwk.alg,
                    jwkKty: jwk.kty,
                    derivedAlgorithm: derivedKey.algorithm.name,
                    derivedLength: derivedKey.algorithm.length,
                    derivedPlaintext: new TextDecoder().decode(new Uint8Array(derivedPlaintext)),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["keyType"], "secret");
    assert_eq!(value["keyAlgorithm"], "AES-CTR");
    assert_eq!(value["keyLength"], 256);
    assert_eq!(value["keyUsages"], "encrypt,decrypt");
    assert_eq!(value["cipherLength"], 7);
    assert_eq!(value["plaintext"], "payload");
    assert_eq!(value["importedPlaintext"], "payload");
    assert_eq!(value["rawLength"], 32);
    assert_eq!(value["jwkAlg"], "A256CTR");
    assert_eq!(value["jwkKty"], "oct");
    assert_eq!(value["derivedAlgorithm"], "AES-CTR");
    assert_eq!(value["derivedLength"], 128);
    assert_eq!(value["derivedPlaintext"], "payload");
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
