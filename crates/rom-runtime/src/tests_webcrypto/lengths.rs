use crate::{RomRuntime, RuntimeConfig};

#[test]
fn validates_webcrypto_aes_key_lengths() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                return {
                    missingGenerateLength: await captureError(() =>
                        crypto.subtle.generateKey(
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    invalidGenerateLength: await captureError(() =>
                        crypto.subtle.generateKey(
                            { name: "AES-GCM", length: 64 },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    invalidRawLength: await captureError(() =>
                        crypto.subtle.importKey(
                            "raw",
                            new Uint8Array(15),
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    mismatchedRawLength: await captureError(() =>
                        crypto.subtle.importKey(
                            "raw",
                            new Uint8Array(16),
                            { name: "AES-GCM", length: 256 },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["missingGenerateLength"]["name"], "TypeError");
    assert_eq!(value["invalidGenerateLength"]["name"], "OperationError");
    assert_eq!(value["invalidRawLength"]["name"], "DataError");
    assert_eq!(value["mismatchedRawLength"]["name"], "DataError");
}

#[test]
fn validates_and_preserves_webcrypto_hmac_key_lengths() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                const imported = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("test"),
                    { name: "HMAC", hash: "SHA-256", length: 24 },
                    true,
                    ["sign", "verify"],
                );
                const importedRaw = await crypto.subtle.exportKey("raw", imported);
                const signature = await crypto.subtle.sign("HMAC", imported, encoder.encode("payload"));
                const verified = await crypto.subtle.verify(
                    "HMAC",
                    imported,
                    signature,
                    encoder.encode("payload"),
                );
                const jwk = await crypto.subtle.exportKey("jwk", imported);

                return {
                    zeroGenerateLength: await captureError(() =>
                        crypto.subtle.generateKey(
                            { name: "HMAC", hash: "SHA-256", length: 0 },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    rawDeclaredTooLong: await captureError(() =>
                        crypto.subtle.importKey(
                            "raw",
                            encoder.encode("test"),
                            { name: "HMAC", hash: "SHA-256", length: 40 },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    jwkDeclaredTooLong: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            jwk,
                            { name: "HMAC", hash: "SHA-256", length: 40 },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    importedLength: imported.algorithm.length,
                    importedRawLength: new Uint8Array(importedRaw).length,
                    verified,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["zeroGenerateLength"]["name"], "OperationError");
    assert_eq!(value["rawDeclaredTooLong"]["name"], "DataError");
    assert_eq!(value["jwkDeclaredTooLong"]["name"], "DataError");
    assert_eq!(value["importedLength"], 24);
    assert_eq!(value["importedRawLength"], 4);
    assert_eq!(value["verified"], true);
}

#[test]
fn validates_webcrypto_derive_key_target_lengths() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("password"),
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const salt = encoder.encode("salt");

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                const derivedDefaultHmac = await crypto.subtle.deriveKey(
                    { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                    baseKey,
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );

                return {
                    missingAesLength: await captureError(() =>
                        crypto.subtle.deriveKey(
                            { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                            baseKey,
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    invalidAesLength: await captureError(() =>
                        crypto.subtle.deriveKey(
                            { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                            baseKey,
                            { name: "AES-GCM", length: 64 },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    invalidHmacLength: await captureError(() =>
                        crypto.subtle.deriveKey(
                            { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                            baseKey,
                            { name: "HMAC", hash: "SHA-256", length: 0 },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    derivedDefaultHmacLength: derivedDefaultHmac.algorithm.length,
                    derivedDefaultHmacHash: derivedDefaultHmac.algorithm.hash.name,
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["missingAesLength"]["name"], "TypeError");
    assert_eq!(value["invalidAesLength"]["name"], "OperationError");
    assert_eq!(value["invalidHmacLength"]["name"], "OperationError");
    assert_eq!(value["derivedDefaultHmacLength"], 512);
    assert_eq!(value["derivedDefaultHmacHash"], "SHA-256");
}

#[test]
fn validates_webcrypto_derive_operation_edges() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const baseKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("password"),
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const salt = encoder.encode("salt");

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                return {
                    nullLength: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                            baseKey,
                            null,
                        ),
                    ),
                    unsupportedAlgorithm: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "AES-GCM", iv: new Uint8Array(12) },
                            baseKey,
                            256,
                        ),
                    ),
                    unsupportedTarget: await captureError(() =>
                        crypto.subtle.deriveKey(
                            { name: "PBKDF2", salt, iterations: 1000, hash: "SHA-256" },
                            baseKey,
                            { name: "PBKDF2" },
                            false,
                            ["deriveBits", "deriveKey"],
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["nullLength"]["name"], "OperationError");
    assert_eq!(value["unsupportedAlgorithm"]["name"], "NotSupportedError");
    assert_eq!(value["unsupportedTarget"]["name"], "NotSupportedError");
}

#[test]
fn validates_webcrypto_unwrap_key_payload_edges() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(12);
                const wrappingKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["wrapKey", "unwrapKey", "encrypt", "decrypt"],
                );

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                const invalidJsonPayload = await crypto.subtle.encrypt(
                    { name: "AES-GCM", iv },
                    wrappingKey,
                    encoder.encode("{bad json"),
                );

                return {
                    invalidFormat: await captureError(() =>
                        crypto.subtle.unwrapKey(
                            "pkcs8",
                            invalidJsonPayload,
                            wrappingKey,
                            { name: "AES-GCM", iv },
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    invalidJwkJson: await captureError(() =>
                        crypto.subtle.unwrapKey(
                            "jwk",
                            invalidJsonPayload,
                            wrappingKey,
                            { name: "AES-GCM", iv },
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["invalidFormat"]["name"], "NotSupportedError");
    assert_eq!(value["invalidJwkJson"]["name"], "DataError");
}
