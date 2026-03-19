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
