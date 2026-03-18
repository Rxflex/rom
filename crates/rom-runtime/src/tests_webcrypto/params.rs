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
fn validates_webcrypto_aes_cbc_and_gcm_params() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const cbcKey = await crypto.subtle.generateKey(
                    { name: "AES-CBC", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const gcmKey = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const payload = new Uint8Array([1, 2, 3]);

                async function captureError(action) {
                    try {
                        await action();
                        return null;
                    } catch (error) {
                        return { name: String(error.name), message: String(error.message) };
                    }
                }

                return {
                    cbcMissingIv: await captureError(() =>
                        crypto.subtle.encrypt({ name: "AES-CBC" }, cbcKey, payload),
                    ),
                    cbcInvalidIv: await captureError(() =>
                        crypto.subtle.encrypt(
                            { name: "AES-CBC", iv: new Uint8Array(15) },
                            cbcKey,
                            payload,
                        ),
                    ),
                    gcmMissingIv: await captureError(() =>
                        crypto.subtle.encrypt({ name: "AES-GCM" }, gcmKey, payload),
                    ),
                    gcmInvalidIv: await captureError(() =>
                        crypto.subtle.encrypt(
                            { name: "AES-GCM", iv: new Uint8Array(11) },
                            gcmKey,
                            payload,
                        ),
                    ),
                    gcmInvalidTagLength: await captureError(() =>
                        crypto.subtle.encrypt(
                            { name: "AES-GCM", iv: new Uint8Array(12), tagLength: 88 },
                            gcmKey,
                            payload,
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["cbcMissingIv"]["name"], "TypeError");
    assert_eq!(value["cbcInvalidIv"]["name"], "OperationError");
    assert_eq!(value["gcmMissingIv"]["name"], "TypeError");
    assert_eq!(value["gcmInvalidIv"]["name"], "OperationError");
    assert_eq!(value["gcmInvalidTagLength"]["name"], "OperationError");
}

#[test]
fn supports_webcrypto_aes_gcm_truncated_tags() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const iv = new Uint8Array(12);
                const payload = encoder.encode("payload");
                const key = await crypto.subtle.generateKey(
                    { name: "AES-GCM", length: 128 },
                    true,
                    ["encrypt", "decrypt"],
                );
                const ciphertext = await crypto.subtle.encrypt(
                    { name: "AES-GCM", iv, tagLength: 96 },
                    key,
                    payload,
                );
                const plaintext = await crypto.subtle.decrypt(
                    { name: "AES-GCM", iv, tagLength: 96 },
                    key,
                    ciphertext,
                );

                return {
                    cipherLength: new Uint8Array(ciphertext).length,
                    plaintext: new TextDecoder().decode(new Uint8Array(plaintext)),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["cipherLength"], 19);
    assert_eq!(value["plaintext"], "payload");
}

#[test]
fn validates_webcrypto_kdf_params() {
    let runtime = RomRuntime::new(RuntimeConfig::default()).unwrap();
    let result = runtime
        .eval_async_as_string(
            r#"
            (async () => {
                const encoder = new TextEncoder();
                const pbkdf2Key = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("password"),
                    "PBKDF2",
                    false,
                    ["deriveBits", "deriveKey"],
                );
                const hkdfKey = await crypto.subtle.importKey(
                    "raw",
                    encoder.encode("input key"),
                    "HKDF",
                    false,
                    ["deriveBits", "deriveKey"],
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
                    pbkdf2MissingSalt: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "PBKDF2", iterations: 1000, hash: "SHA-256" },
                            pbkdf2Key,
                            256,
                        ),
                    ),
                    pbkdf2MissingHash: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "PBKDF2", salt: encoder.encode("salt"), iterations: 1000 },
                            pbkdf2Key,
                            256,
                        ),
                    ),
                    pbkdf2InvalidIterations: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "PBKDF2", salt: encoder.encode("salt"), iterations: 0, hash: "SHA-256" },
                            pbkdf2Key,
                            256,
                        ),
                    ),
                    hkdfMissingInfo: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "HKDF", salt: encoder.encode("salt"), hash: "SHA-256" },
                            hkdfKey,
                            256,
                        ),
                    ),
                    hkdfMissingHash: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "HKDF", salt: encoder.encode("salt"), info: encoder.encode("info") },
                            hkdfKey,
                            256,
                        ),
                    ),
                    invalidLength: await captureError(() =>
                        crypto.subtle.deriveBits(
                            { name: "HKDF", salt: encoder.encode("salt"), info: encoder.encode("info"), hash: "SHA-256" },
                            hkdfKey,
                            130,
                        ),
                    ),
                };
            })()
            "#,
        )
        .unwrap();

    let value: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(value["pbkdf2MissingSalt"]["name"], "TypeError");
    assert_eq!(value["pbkdf2MissingHash"]["name"], "TypeError");
    assert_eq!(value["pbkdf2InvalidIterations"]["name"], "OperationError");
    assert_eq!(value["hkdfMissingInfo"]["name"], "TypeError");
    assert_eq!(value["hkdfMissingHash"]["name"], "TypeError");
    assert_eq!(value["invalidLength"]["name"], "OperationError");
}
