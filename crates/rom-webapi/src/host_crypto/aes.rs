use aes::{
    Aes128, Aes192, Aes256,
    cipher::{
        BlockEncrypt, BlockSizeUser, KeyInit as BlockKeyInit,
        generic_array::{GenericArray, typenum::U16},
    },
};
use aes_kw::{KekAes128, KekAes192, KekAes256};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use cbc::{
    Decryptor as CbcDecryptor, Encryptor as CbcEncryptor,
    cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7},
};

use super::{
    ops::validate_usages,
    types::{ExportedJwk, JsonWebKey, KeyAlgorithm},
};

pub(crate) fn validate_aes_ctr_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(
        usages,
        &["encrypt", "decrypt", "wrapKey", "unwrapKey"],
        "AES-CTR",
    )
}

pub(crate) fn validate_aes_cbc_usages(usages: &[String]) -> Result<(), String> {
    validate_usages(
        usages,
        &["encrypt", "decrypt", "wrapKey", "unwrapKey"],
        "AES-CBC",
    )
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

pub(crate) fn build_aes_algorithm(name: &str, secret_len: usize) -> KeyAlgorithm {
    KeyAlgorithm {
        name: name.to_owned(),
        hash: None,
        length: Some(secret_len * 8),
    }
}

pub(crate) fn normalize_aes_length(length: usize, algorithm: &str) -> Result<usize, String> {
    match (algorithm.to_ascii_uppercase().as_str(), length) {
        ("AES-CTR", 128 | 192 | 256)
        | ("AES-CBC", 128 | 192 | 256)
        | ("AES-GCM", 128 | 192 | 256)
        | ("AES-KW", 128 | 192 | 256) => Ok(length / 8),
        (_, other) => Err(format!("Unsupported {algorithm} length: {other}")),
    }
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

    if let Some(actual_alg) = jwk.alg.as_deref()
        && actual_alg != expected_alg
    {
        return Err(format!(
            "JWK alg mismatch: expected {expected_alg}, got {actual_alg}"
        ));
    }

    Ok(secret)
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

pub(crate) fn encrypt_aes_ctr(
    secret: &[u8],
    counter: &[u8],
    counter_length: usize,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    match secret.len() {
        16 => apply_aes_ctr::<Aes128>(secret, counter, counter_length, data),
        24 => apply_aes_ctr::<Aes192>(secret, counter, counter_length, data),
        32 => apply_aes_ctr::<Aes256>(secret, counter, counter_length, data),
        other => Err(format!(
            "Unsupported AES-CTR raw key length: {} bits",
            other * 8
        )),
    }
}

pub(crate) fn decrypt_aes_ctr(
    secret: &[u8],
    counter: &[u8],
    counter_length: usize,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    encrypt_aes_ctr(secret, counter, counter_length, data)
}

pub(crate) fn encrypt_aes_cbc(secret: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    prepare_cbc_iv(iv)?;
    let mut out = vec![0_u8; data.len() + 16];

    let ciphertext = match secret.len() {
        16 => CbcEncryptor::<Aes128>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .encrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC encryption failed".to_owned())?,
        24 => CbcEncryptor::<Aes192>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .encrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC encryption failed".to_owned())?,
        32 => CbcEncryptor::<Aes256>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .encrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC encryption failed".to_owned())?,
        other => {
            return Err(format!(
                "Unsupported AES-CBC raw key length: {} bits",
                other * 8
            ));
        }
    };

    Ok(ciphertext.to_vec())
}

pub(crate) fn decrypt_aes_cbc(secret: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    prepare_cbc_iv(iv)?;
    let mut out = vec![0_u8; data.len()];

    let plaintext = match secret.len() {
        16 => CbcDecryptor::<Aes128>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .decrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC decryption failed".to_owned())?,
        24 => CbcDecryptor::<Aes192>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .decrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC decryption failed".to_owned())?,
        32 => CbcDecryptor::<Aes256>::new_from_slices(secret, iv)
            .map_err(|error| error.to_string())?
            .decrypt_padded_b2b_mut::<Pkcs7>(data, &mut out)
            .map_err(|_| "OperationError: AES-CBC decryption failed".to_owned())?,
        other => {
            return Err(format!(
                "Unsupported AES-CBC raw key length: {} bits",
                other * 8
            ));
        }
    };

    Ok(plaintext.to_vec())
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

fn apply_aes_ctr<Cipher>(
    secret: &[u8],
    counter: &[u8],
    counter_length: usize,
    data: &[u8],
) -> Result<Vec<u8>, String>
where
    Cipher: BlockEncrypt + BlockKeyInit + BlockSizeUser<BlockSize = U16>,
{
    let counter_length = normalize_ctr_length(counter_length)?;
    let mut counter_block = prepare_ctr_counter(counter)?;
    ensure_ctr_capacity(counter_block, counter_length, data.len())?;
    let cipher = Cipher::new_from_slice(secret).map_err(|error| error.to_string())?;
    let mut output = Vec::with_capacity(data.len());

    for chunk in data.chunks(16) {
        let mut keystream = GenericArray::clone_from_slice(&counter_block);
        cipher.encrypt_block(&mut keystream);

        output.extend(
            chunk
                .iter()
                .zip(keystream.iter())
                .map(|(plain, mask)| plain ^ mask),
        );

        increment_ctr_counter(&mut counter_block, counter_length);
    }

    Ok(output)
}

fn normalize_ctr_length(counter_length: usize) -> Result<usize, String> {
    match counter_length {
        1..=128 => Ok(counter_length),
        other => Err(format!("Unsupported AES-CTR length: {other}")),
    }
}

fn prepare_cbc_iv(iv: &[u8]) -> Result<(), String> {
    if iv.len() != 16 {
        return Err(format!(
            "Unsupported AES-CBC iv length: expected 16 bytes, got {}",
            iv.len()
        ));
    }
    Ok(())
}

fn prepare_ctr_counter(counter: &[u8]) -> Result<[u8; 16], String> {
    if counter.len() != 16 {
        return Err(format!(
            "Unsupported AES-CTR counter length: expected 16 bytes, got {}",
            counter.len()
        ));
    }

    let mut block = [0_u8; 16];
    block.copy_from_slice(counter);
    Ok(block)
}

fn ensure_ctr_capacity(
    counter_block: [u8; 16],
    counter_length: usize,
    data_len: usize,
) -> Result<(), String> {
    let blocks = data_len.div_ceil(16) as u128;
    if blocks == 0 {
        return Ok(());
    }

    let counter = u128::from_be_bytes(counter_block);
    if counter_length == 128 {
        if (blocks - 1) > (u128::MAX - counter) {
            return Err("OperationError: AES-CTR counter would wrap".to_owned());
        }
        return Ok(());
    }

    let space = 1_u128 << counter_length;
    let current = counter & low_mask(counter_length);
    if blocks > (space - current) {
        return Err("OperationError: AES-CTR counter would wrap".to_owned());
    }
    Ok(())
}

fn increment_ctr_counter(counter_block: &mut [u8; 16], counter_length: usize) {
    let counter = u128::from_be_bytes(*counter_block);
    let mask = low_mask(counter_length);
    let next = (counter & !mask) | ((counter & mask).wrapping_add(1) & mask);
    *counter_block = next.to_be_bytes();
}

fn low_mask(bits: usize) -> u128 {
    if bits == 128 {
        u128::MAX
    } else {
        (1_u128 << bits) - 1
    }
}

fn jwk_alg_for_aes(algorithm: &str, secret_len: usize) -> Option<&'static str> {
    match (algorithm.to_ascii_uppercase().as_str(), secret_len) {
        ("AES-CTR", 16) => Some("A128CTR"),
        ("AES-CTR", 24) => Some("A192CTR"),
        ("AES-CTR", 32) => Some("A256CTR"),
        ("AES-CBC", 16) => Some("A128CBC"),
        ("AES-CBC", 24) => Some("A192CBC"),
        ("AES-CBC", 32) => Some("A256CBC"),
        ("AES-GCM", 16) => Some("A128GCM"),
        ("AES-GCM", 24) => Some("A192GCM"),
        ("AES-GCM", 32) => Some("A256GCM"),
        ("AES-KW", 16) => Some("A128KW"),
        ("AES-KW", 24) => Some("A192KW"),
        ("AES-KW", 32) => Some("A256KW"),
        _ => None,
    }
}
