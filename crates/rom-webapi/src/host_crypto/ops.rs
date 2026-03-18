use aes_gcm::{
    Aes128Gcm, Aes256Gcm, KeyInit,
    aead::{Aead, Payload, generic_array::GenericArray},
};
use aes_kw::{KekAes128, KekAes192, KekAes256};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use typenum::U12;

use super::types::{
    AlgorithmDescriptor, ExportedJwk, HashAlgorithm, JsonWebKey, KeyAlgorithm, KeyHashAlgorithm,
};

pub(crate) fn parse_hash_from_descriptor(
    descriptor: &AlgorithmDescriptor,
) -> Result<HashAlgorithm, String> {
    parse_hash_algorithm(
        descriptor
            .hash
            .as_deref()
            .ok_or_else(|| "Missing algorithm.hash".to_owned())?,
    )
}

pub(crate) fn parse_hash_algorithm(name: &str) -> Result<HashAlgorithm, String> {
    match name.to_ascii_uppercase().as_str() {
        "SHA-1" => Ok(HashAlgorithm::Sha1),
        "SHA-256" => Ok(HashAlgorithm::Sha256),
        "SHA-384" => Ok(HashAlgorithm::Sha384),
        "SHA-512" => Ok(HashAlgorithm::Sha512),
        other => Err(format!("Unsupported hash algorithm: {other}")),
    }
}

pub(crate) fn ensure_algorithm_name(actual: &str, expected: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    Err(format!("Unsupported algorithm: {actual}"))
}

pub(crate) fn validate_hmac_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(usages, &["sign", "verify"], "HMAC")
}

pub(crate) fn validate_aes_gcm_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(
        usages,
        &["encrypt", "decrypt", "wrapKey", "unwrapKey"],
        "AES-GCM",
    )
}

pub(crate) fn validate_aes_kw_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(usages, &["wrapKey", "unwrapKey"], "AES-KW")
}

pub(crate) fn validate_pbkdf2_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(usages, &["deriveBits", "deriveKey"], "PBKDF2")
}

pub(crate) fn validate_hkdf_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(usages, &["deriveBits", "deriveKey"], "HKDF")
}

pub(crate) fn digest_bytes(algorithm: HashAlgorithm, data: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Sha1 => Sha1::digest(data).to_vec(),
        HashAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
        HashAlgorithm::Sha384 => Sha384::digest(data).to_vec(),
        HashAlgorithm::Sha512 => Sha512::digest(data).to_vec(),
    }
}

pub(crate) fn build_hmac_algorithm(hash: HashAlgorithm, secret_len: usize) -> KeyAlgorithm {
    KeyAlgorithm {
        name: "HMAC".to_owned(),
        hash: Some(KeyHashAlgorithm {
            name: hash.web_name().to_owned(),
        }),
        length: Some(secret_len * 8),
    }
}

pub(crate) fn build_aes_algorithm(name: &str, secret_len: usize) -> KeyAlgorithm {
    KeyAlgorithm {
        name: name.to_owned(),
        hash: None,
        length: Some(secret_len * 8),
    }
}

pub(crate) fn build_pbkdf2_algorithm() -> KeyAlgorithm {
    KeyAlgorithm {
        name: "PBKDF2".to_owned(),
        hash: None,
        length: None,
    }
}

pub(crate) fn build_hkdf_algorithm() -> KeyAlgorithm {
    KeyAlgorithm {
        name: "HKDF".to_owned(),
        hash: None,
        length: None,
    }
}

pub(crate) fn default_hmac_key_length(hash: HashAlgorithm) -> usize {
    match hash {
        HashAlgorithm::Sha1 => 64,
        HashAlgorithm::Sha256 => 64,
        HashAlgorithm::Sha384 => 128,
        HashAlgorithm::Sha512 => 128,
    }
}

pub(crate) fn normalize_aes_length(length: usize, algorithm: &str) -> Result<usize, String> {
    match (algorithm.to_ascii_uppercase().as_str(), length) {
        ("AES-GCM", 128 | 256) | ("AES-KW", 128 | 192 | 256) => Ok(length / 8),
        (_, other) => Err(format!("Unsupported {algorithm} length: {other}")),
    }
}

pub(crate) fn import_hmac_jwk(
    value: serde_json::Value,
    hash: HashAlgorithm,
) -> Result<Vec<u8>, String> {
    import_jwk_oct_secret(value, Some(hash.jwk_alg_name()), "HMAC")
}

pub(crate) fn import_aes_jwk(value: serde_json::Value, algorithm: &str) -> Result<Vec<u8>, String> {
    let jwk: JsonWebKey = serde_json::from_value(value).map_err(|error| error.to_string())?;
    if jwk.kty != "oct" {
        return Err(format!("Unsupported JWK kty for {algorithm}"));
    }

    let secret = URL_SAFE_NO_PAD
        .decode(jwk.k.as_bytes())
        .map_err(|error| error.to_string())?;
    let expected_alg = jwk_alg_for_aes(algorithm, secret.len()).ok_or_else(|| {
        format!(
            "Unsupported {algorithm} raw key length: {} bits",
            secret.len() * 8
        )
    })?;

    if let Some(actual_alg) = jwk.alg.as_deref() {
        if actual_alg != expected_alg {
            return Err(format!(
                "JWK alg mismatch: expected {expected_alg}, got {actual_alg}"
            ));
        }
    }

    Ok(secret)
}

pub(crate) fn sign_hmac(
    hash: HashAlgorithm,
    secret: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, String> {
    match hash {
        HashAlgorithm::Sha1 => {
            let mut mac =
                <Hmac<Sha1> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        HashAlgorithm::Sha256 => {
            let mut mac =
                <Hmac<Sha256> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        HashAlgorithm::Sha384 => {
            let mut mac =
                <Hmac<Sha384> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        HashAlgorithm::Sha512 => {
            let mut mac =
                <Hmac<Sha512> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.finalize().into_bytes().to_vec())
        }
    }
}

pub(crate) fn verify_hmac(
    hash: HashAlgorithm,
    secret: &[u8],
    data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    match hash {
        HashAlgorithm::Sha1 => {
            let mut mac =
                <Hmac<Sha1> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.verify_slice(signature).is_ok())
        }
        HashAlgorithm::Sha256 => {
            let mut mac =
                <Hmac<Sha256> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.verify_slice(signature).is_ok())
        }
        HashAlgorithm::Sha384 => {
            let mut mac =
                <Hmac<Sha384> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.verify_slice(signature).is_ok())
        }
        HashAlgorithm::Sha512 => {
            let mut mac =
                <Hmac<Sha512> as Mac>::new_from_slice(secret).map_err(|error| error.to_string())?;
            mac.update(data);
            Ok(mac.verify_slice(signature).is_ok())
        }
    }
}

pub(crate) fn encrypt_aes_gcm(
    secret: &[u8],
    iv: &[u8],
    additional_data: &[u8],
    data: &[u8],
    tag_length: Option<usize>,
) -> Result<Vec<u8>, String> {
    let nonce = prepare_nonce(iv)?;
    let tag_length = normalize_tag_length(tag_length)?;

    match secret.len() {
        16 if tag_length == 128 => {
            let cipher = Aes128Gcm::new_from_slice(secret).map_err(|error| error.to_string())?;
            cipher
                .encrypt(
                    nonce,
                    Payload {
                        msg: data,
                        aad: additional_data,
                    },
                )
                .map_err(|_| "OperationError: AES-GCM encryption failed".to_owned())
        }
        32 if tag_length == 128 => {
            let cipher = Aes256Gcm::new_from_slice(secret).map_err(|error| error.to_string())?;
            cipher
                .encrypt(
                    nonce,
                    Payload {
                        msg: data,
                        aad: additional_data,
                    },
                )
                .map_err(|_| "OperationError: AES-GCM encryption failed".to_owned())
        }
        16 | 32 => Err(format!("Unsupported AES-GCM tagLength: {tag_length}")),
        other => Err(format!(
            "Unsupported AES-GCM raw key length: {} bits",
            other * 8
        )),
    }
}

pub(crate) fn decrypt_aes_gcm(
    secret: &[u8],
    iv: &[u8],
    additional_data: &[u8],
    data: &[u8],
    tag_length: Option<usize>,
) -> Result<Vec<u8>, String> {
    let nonce = prepare_nonce(iv)?;
    let tag_length = normalize_tag_length(tag_length)?;

    match secret.len() {
        16 if tag_length == 128 => {
            let cipher = Aes128Gcm::new_from_slice(secret).map_err(|error| error.to_string())?;
            cipher
                .decrypt(
                    nonce,
                    Payload {
                        msg: data,
                        aad: additional_data,
                    },
                )
                .map_err(|_| "OperationError: AES-GCM decryption failed".to_owned())
        }
        32 if tag_length == 128 => {
            let cipher = Aes256Gcm::new_from_slice(secret).map_err(|error| error.to_string())?;
            cipher
                .decrypt(
                    nonce,
                    Payload {
                        msg: data,
                        aad: additional_data,
                    },
                )
                .map_err(|_| "OperationError: AES-GCM decryption failed".to_owned())
        }
        16 | 32 => Err(format!("Unsupported AES-GCM tagLength: {tag_length}")),
        other => Err(format!(
            "Unsupported AES-GCM raw key length: {} bits",
            other * 8
        )),
    }
}

pub(crate) fn encrypt_aes_kw(secret: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    match secret.len() {
        16 => {
            let kek = KekAes128::try_from(secret).map_err(|error| error.to_string())?;
            let mut out = vec![0_u8; data.len() + aes_kw::IV_LEN];
            kek.wrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW wrap failed".to_owned())?;
            Ok(out)
        }
        24 => {
            let kek = KekAes192::try_from(secret).map_err(|error| error.to_string())?;
            let mut out = vec![0_u8; data.len() + aes_kw::IV_LEN];
            kek.wrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW wrap failed".to_owned())?;
            Ok(out)
        }
        32 => {
            let kek = KekAes256::try_from(secret).map_err(|error| error.to_string())?;
            let mut out = vec![0_u8; data.len() + aes_kw::IV_LEN];
            kek.wrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW wrap failed".to_owned())?;
            Ok(out)
        }
        other => Err(format!(
            "Unsupported AES-KW raw key length: {} bits",
            other * 8
        )),
    }
}

pub(crate) fn decrypt_aes_kw(secret: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    match secret.len() {
        16 => {
            let kek = KekAes128::try_from(secret).map_err(|error| error.to_string())?;
            let out_len = data
                .len()
                .checked_sub(aes_kw::IV_LEN)
                .ok_or_else(|| "OperationError: AES-KW unwrap failed".to_owned())?;
            let mut out = vec![0_u8; out_len];
            kek.unwrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW unwrap failed".to_owned())?;
            Ok(out)
        }
        24 => {
            let kek = KekAes192::try_from(secret).map_err(|error| error.to_string())?;
            let out_len = data
                .len()
                .checked_sub(aes_kw::IV_LEN)
                .ok_or_else(|| "OperationError: AES-KW unwrap failed".to_owned())?;
            let mut out = vec![0_u8; out_len];
            kek.unwrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW unwrap failed".to_owned())?;
            Ok(out)
        }
        32 => {
            let kek = KekAes256::try_from(secret).map_err(|error| error.to_string())?;
            let out_len = data
                .len()
                .checked_sub(aes_kw::IV_LEN)
                .ok_or_else(|| "OperationError: AES-KW unwrap failed".to_owned())?;
            let mut out = vec![0_u8; out_len];
            kek.unwrap(data, &mut out)
                .map_err(|_| "OperationError: AES-KW unwrap failed".to_owned())?;
            Ok(out)
        }
        other => Err(format!(
            "Unsupported AES-KW raw key length: {} bits",
            other * 8
        )),
    }
}

pub(crate) fn derive_pbkdf2_bits(
    secret: &[u8],
    salt: &[u8],
    iterations: u32,
    hash: HashAlgorithm,
    length: usize,
) -> Result<Vec<u8>, String> {
    if iterations == 0 {
        return Err("PBKDF2 iterations must be greater than zero".to_owned());
    }
    if !length.is_multiple_of(8) {
        return Err("PBKDF2 deriveBits length must be a multiple of 8".to_owned());
    }

    let mut bytes = vec![0_u8; length / 8];
    match hash {
        HashAlgorithm::Sha1 => pbkdf2_hmac::<Sha1>(secret, salt, iterations, &mut bytes),
        HashAlgorithm::Sha256 => pbkdf2_hmac::<Sha256>(secret, salt, iterations, &mut bytes),
        HashAlgorithm::Sha384 => pbkdf2_hmac::<Sha384>(secret, salt, iterations, &mut bytes),
        HashAlgorithm::Sha512 => pbkdf2_hmac::<Sha512>(secret, salt, iterations, &mut bytes),
    }
    Ok(bytes)
}

pub(crate) fn derive_hkdf_bits(
    secret: &[u8],
    salt: &[u8],
    info: &[u8],
    hash: HashAlgorithm,
    length: usize,
) -> Result<Vec<u8>, String> {
    if !length.is_multiple_of(8) {
        return Err("HKDF deriveBits length must be a multiple of 8".to_owned());
    }

    let mut bytes = vec![0_u8; length / 8];
    match hash {
        HashAlgorithm::Sha1 => Hkdf::<Sha1>::new(Some(salt), secret)
            .expand(info, &mut bytes)
            .map_err(|_| "OperationError: HKDF expansion failed".to_owned())?,
        HashAlgorithm::Sha256 => Hkdf::<Sha256>::new(Some(salt), secret)
            .expand(info, &mut bytes)
            .map_err(|_| "OperationError: HKDF expansion failed".to_owned())?,
        HashAlgorithm::Sha384 => Hkdf::<Sha384>::new(Some(salt), secret)
            .expand(info, &mut bytes)
            .map_err(|_| "OperationError: HKDF expansion failed".to_owned())?,
        HashAlgorithm::Sha512 => Hkdf::<Sha512>::new(Some(salt), secret)
            .expand(info, &mut bytes)
            .map_err(|_| "OperationError: HKDF expansion failed".to_owned())?,
    }
    Ok(bytes)
}

pub(crate) fn export_hmac_jwk(
    secret: &[u8],
    hash: HashAlgorithm,
    extractable: bool,
    usages: Vec<String>,
) -> ExportedJwk {
    ExportedJwk {
        kty: "oct",
        k: URL_SAFE_NO_PAD.encode(secret),
        alg: Some(hash.jwk_alg_name()),
        ext: extractable,
        key_ops: usages,
    }
}

pub(crate) fn export_aes_jwk(
    secret: &[u8],
    algorithm: &str,
    extractable: bool,
    usages: Vec<String>,
) -> Result<ExportedJwk, String> {
    let alg = jwk_alg_for_aes(algorithm, secret.len()).ok_or_else(|| {
        format!(
            "Unsupported {algorithm} raw key length: {} bits",
            secret.len() * 8
        )
    })?;
    Ok(ExportedJwk {
        kty: "oct",
        k: URL_SAFE_NO_PAD.encode(secret),
        alg: Some(alg),
        ext: extractable,
        key_ops: usages,
    })
}

fn validate_usages(usages: &[String], allowed: &[&str], algorithm: &str) -> Result<(), String> {
    if usages.is_empty() {
        return Err(format!(
            "SyntaxError: {algorithm} keys require at least one usage"
        ));
    }
    for usage in usages {
        if allowed.iter().any(|allowed_usage| usage == allowed_usage) {
            continue;
        }
        return Err(format!("Invalid key usage for {algorithm}: {usage}"));
    }
    Ok(())
}

fn import_jwk_oct_secret(
    value: serde_json::Value,
    expected_alg: Option<&str>,
    label: &str,
) -> Result<Vec<u8>, String> {
    let jwk: JsonWebKey = serde_json::from_value(value).map_err(|error| error.to_string())?;
    if jwk.kty != "oct" {
        return Err(format!("Unsupported JWK kty for {label}"));
    }
    if let Some(expected) = expected_alg {
        if let Some(alg) = jwk.alg.as_deref() {
            if alg != expected {
                return Err(format!("JWK alg mismatch: expected {expected}, got {alg}"));
            }
        }
    }
    URL_SAFE_NO_PAD
        .decode(jwk.k.as_bytes())
        .map_err(|error| error.to_string())
}

fn normalize_tag_length(tag_length: Option<usize>) -> Result<usize, String> {
    match tag_length.unwrap_or(128) {
        128 => Ok(128),
        other => Err(format!("Unsupported AES-GCM tagLength: {other}")),
    }
}

fn prepare_nonce(iv: &[u8]) -> Result<&GenericArray<u8, U12>, String> {
    if iv.len() != 12 {
        return Err(format!(
            "Unsupported AES-GCM iv length: expected 12 bytes, got {}",
            iv.len()
        ));
    }
    Ok(GenericArray::from_slice(iv))
}

fn jwk_alg_for_aes(algorithm: &str, secret_len: usize) -> Option<&'static str> {
    match (algorithm.to_ascii_uppercase().as_str(), secret_len) {
        ("AES-GCM", 16) => Some("A128GCM"),
        ("AES-GCM", 32) => Some("A256GCM"),
        ("AES-KW", 16) => Some("A128KW"),
        ("AES-KW", 24) => Some("A192KW"),
        ("AES-KW", 32) => Some("A256KW"),
        _ => None,
    }
}
