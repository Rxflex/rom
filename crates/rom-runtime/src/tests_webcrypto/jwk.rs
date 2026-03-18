use crate::{RomRuntime, RuntimeConfig};

#[test]
fn validates_webcrypto_jwk_import_content() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const hmacKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("secret"),
                    { name: "HMAC", hash: "SHA-256" },
                    true,
                    ["sign", "verify"],
                );
                const aesKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const hmacJwk = await crypto.subtle.exportKey("jwk", hmacKey);
                const aesJwk = await crypto.subtle.exportKey("jwk", aesKey);

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                const missingK = { ...hmacJwk };
                delete missingK.k;

                return {
                    invalidKty: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            { ...hmacJwk, kty: "RSA" },
                            { name: "HMAC", hash: "SHA-256" },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    missingK: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            missingK,
                            { name: "HMAC", hash: "SHA-256" },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    algMismatch: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            { ...aesJwk, alg: "A256GCM" },
                            { name: "AES-GCM" },
                            true,
                            ["encrypt", "decrypt"],
                        ),
                    ),
                    useMismatch: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            { ...hmacJwk, use: "enc" },
                            { name: "HMAC", hash: "SHA-256" },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    keyOpsMismatch: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            { ...hmacJwk, key_ops: ["sign"] },
                            { name: "HMAC", hash: "SHA-256" },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    extMismatch: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            { ...hmacJwk, ext: false },
                            { name: "HMAC", hash: "SHA-256" },
                            true,
                            ["sign", "verify"],
                        ),
                    ),
                    unsupportedPbkdf2Jwk: await captureError(() =>
                        crypto.subtle.importKey(
                            "jwk",
                            hmacJwk,
                            "PBKDF2",
                            false,
                            ["deriveBits"],
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["invalidKty"]["name"], "DataError");
    assert_eq!(value["missingK"]["name"], "DataError");
    assert_eq!(value["algMismatch"]["name"], "DataError");
    assert_eq!(value["useMismatch"]["name"], "DataError");
    assert_eq!(value["keyOpsMismatch"]["name"], "DataError");
    assert_eq!(value["extMismatch"]["name"], "DataError");
    assert_eq!(value["unsupportedPbkdf2Jwk"]["name"], "NotSupportedError");
}
