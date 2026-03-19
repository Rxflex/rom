use crate::{RomRuntime, RuntimeConfig};

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

#[test]
fn validates_webcrypto_wrap_and_unwrap_algorithms() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const keyToWrap = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const wrappingKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["wrapKey", "unwrapKey"],
                );

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                return {
                    wrapUnsupported: await captureError(() =>
                        crypto.subtle.wrapKey(
                            "raw",
                            keyToWrap,
                            wrappingKey,
                            "PBKDF2",
                        ),
                    ),
                    unwrapUnsupported: await captureError(() =>
                        crypto.subtle.unwrapKey(
                            "raw",
                            new Uint8Array([1, 2, 3]).buffer,
                            wrappingKey,
                            "PBKDF2",
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

    assert_eq!(value["wrapUnsupported"]["name"], "NotSupportedError");
    assert_eq!(value["unwrapUnsupported"]["name"], "NotSupportedError");
}

#[test]
fn validates_webcrypto_aes_kw_payload_lengths() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const wrappingKey = await crypto.subtle.generateKey(
                    { name: "AES-KW", length: 128 },
                    true,
                    ["wrapKey", "unwrapKey"],
                );
                const hmacKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("secret"),
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                return {
                    wrapShortPayload: await captureError(() =>
                        crypto.subtle.wrapKey("raw", hmacKey, wrappingKey, "AES-KW"),
                    ),
                    unwrapShortPayload: await captureError(() =>
                        crypto.subtle.unwrapKey(
                            "raw",
                            new Uint8Array(15),
                            wrappingKey,
                            "AES-KW",
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

    assert_eq!(value["wrapShortPayload"]["name"], "OperationError");
    assert_eq!(value["unwrapShortPayload"]["name"], "OperationError");
}
