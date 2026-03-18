mod ops;
mod types;

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde_json::Value;

use self::{
    ops::{
        build_aes_algorithm, build_hmac_algorithm, build_pbkdf2_algorithm, default_hmac_key_length,
        derive_pbkdf2_bits, digest_bytes, encrypt_aes_gcm, ensure_algorithm_name, export_aes_jwk,
        export_hmac_jwk, import_aes_jwk, import_hmac_jwk, normalize_aes_length,
        parse_hash_algorithm, parse_hash_from_descriptor, sign_hmac, validate_aes_usages,
        validate_hmac_usages, validate_pbkdf2_usages, verify_hmac,
    },
    types::{
        BytesPayload, CryptoKeyRecord, DecryptPayload, DeriveBitsPayload, DigestPayload,
        EncryptPayload, ExportKeyPayload, GenerateKeyPayload, ImportKeyPayload, KeyDescriptor,
        KeyMaterial, SignPayload, VerifyPayload, VerifyResultPayload,
    },
};

#[derive(Clone, Default)]
pub struct CryptoHost {
    next_key_id: Arc<AtomicU64>,
    store: Arc<Mutex<HashMap<String, CryptoKeyRecord>>>,
}

impl CryptoHost {
    pub fn random_bytes_json(&self, length: usize) -> Result<String, String> {
        let mut bytes = vec![0_u8; length];
        getrandom::fill(&mut bytes).map_err(|error| error.to_string())?;
        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub fn subtle_digest(&self, payload: &str) -> Result<String, String> {
        let payload: DigestPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let algorithm = parse_hash_algorithm(&payload.algorithm)?;
        let bytes = digest_bytes(algorithm, &payload.data);
        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub fn subtle_generate_key(&self, payload: &str) -> Result<String, String> {
        let payload: GenerateKeyPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;

        match payload.algorithm.name.to_ascii_uppercase().as_str() {
            "HMAC" => {
                validate_hmac_usages(&payload.usages)?;
                let hash = parse_hash_from_descriptor(&payload.algorithm)?;
                let byte_length = payload
                    .algorithm
                    .length
                    .map(|length| length.div_ceil(8))
                    .unwrap_or_else(|| default_hmac_key_length(hash));
                let mut secret = vec![0_u8; byte_length];
                getrandom::fill(&mut secret).map_err(|error| error.to_string())?;

                self.store_key(CryptoKeyRecord {
                    extractable: payload.extractable,
                    key_type: "secret".to_owned(),
                    algorithm: build_hmac_algorithm(hash, secret.len()),
                    usages: payload.usages,
                    material: KeyMaterial::Hmac { secret, hash },
                })
            }
            "AES-GCM" => {
                validate_aes_usages(&payload.usages)?;
                let byte_length = normalize_aes_length(
                    payload
                        .algorithm
                        .length
                        .ok_or_else(|| "Missing algorithm.length".to_owned())?,
                )?;
                let mut secret = vec![0_u8; byte_length];
                getrandom::fill(&mut secret).map_err(|error| error.to_string())?;

                self.store_key(CryptoKeyRecord {
                    extractable: payload.extractable,
                    key_type: "secret".to_owned(),
                    algorithm: build_aes_algorithm(secret.len()),
                    usages: payload.usages,
                    material: KeyMaterial::Aes { secret },
                })
            }
            other => Err(format!("Unsupported algorithm: {other}")),
        }
    }

    pub fn subtle_import_key(&self, payload: &str) -> Result<String, String> {
        let payload: ImportKeyPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;

        match payload.algorithm.name.to_ascii_uppercase().as_str() {
            "HMAC" => {
                validate_hmac_usages(&payload.usages)?;
                let hash = parse_hash_from_descriptor(&payload.algorithm)?;
                let secret = match payload.format.as_str() {
                    "raw" => deserialize_raw_bytes(payload.key_data)?,
                    "jwk" => import_hmac_jwk(payload.key_data, hash)?,
                    other => return Err(format!("Unsupported key import format: {other}")),
                };

                self.store_key(CryptoKeyRecord {
                    extractable: payload.extractable,
                    key_type: "secret".to_owned(),
                    algorithm: build_hmac_algorithm(hash, secret.len()),
                    usages: payload.usages,
                    material: KeyMaterial::Hmac { secret, hash },
                })
            }
            "AES-GCM" => {
                validate_aes_usages(&payload.usages)?;
                let secret = match payload.format.as_str() {
                    "raw" => deserialize_raw_bytes(payload.key_data)?,
                    "jwk" => import_aes_jwk(payload.key_data)?,
                    other => return Err(format!("Unsupported key import format: {other}")),
                };
                let _ = normalize_aes_length(secret.len() * 8)?;

                self.store_key(CryptoKeyRecord {
                    extractable: payload.extractable,
                    key_type: "secret".to_owned(),
                    algorithm: build_aes_algorithm(secret.len()),
                    usages: payload.usages,
                    material: KeyMaterial::Aes { secret },
                })
            }
            "PBKDF2" => {
                validate_pbkdf2_usages(&payload.usages)?;
                if payload.format != "raw" {
                    return Err(format!(
                        "Unsupported key import format for PBKDF2: {}",
                        payload.format
                    ));
                }
                let secret = deserialize_raw_bytes(payload.key_data)?;

                self.store_key(CryptoKeyRecord {
                    extractable: payload.extractable,
                    key_type: "secret".to_owned(),
                    algorithm: build_pbkdf2_algorithm(),
                    usages: payload.usages,
                    material: KeyMaterial::Pbkdf2 { secret },
                })
            }
            other => Err(format!("Unsupported algorithm: {other}")),
        }
    }

    pub fn subtle_export_key(&self, payload: &str) -> Result<String, String> {
        let payload: ExportKeyPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let record = self.get_key(&payload.key_id)?;

        if !record.extractable {
            return Err("InvalidAccessError: key is not extractable".to_owned());
        }

        match payload.format.as_str() {
            "raw" => {
                let bytes = match &record.material {
                    KeyMaterial::Hmac { secret, .. }
                    | KeyMaterial::Aes { secret }
                    | KeyMaterial::Pbkdf2 { secret } => secret.clone(),
                };
                serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
            }
            "jwk" => {
                let exported = match &record.material {
                    KeyMaterial::Hmac { secret, hash } => serde_json::to_value(export_hmac_jwk(
                        secret,
                        *hash,
                        record.extractable,
                        record.usages.clone(),
                    ))
                    .map_err(|error| error.to_string())?,
                    KeyMaterial::Aes { secret } => serde_json::to_value(export_aes_jwk(
                        secret,
                        record.extractable,
                        record.usages.clone(),
                    )?)
                    .map_err(|error| error.to_string())?,
                    KeyMaterial::Pbkdf2 { .. } => {
                        return Err("Unsupported key export format for PBKDF2: jwk".to_owned());
                    }
                };
                serde_json::to_string(&exported).map_err(|error| error.to_string())
            }
            other => Err(format!("Unsupported key export format: {other}")),
        }
    }

    pub fn subtle_sign(&self, payload: &str) -> Result<String, String> {
        let payload: SignPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        ensure_algorithm_name(&payload.algorithm.name, "HMAC")?;
        let record = self.get_key(&payload.key_id)?;

        if !record.usages.iter().any(|usage| usage == "sign") {
            return Err("InvalidAccessError: key does not allow sign".to_owned());
        }

        let bytes = match &record.material {
            KeyMaterial::Hmac { secret, hash } => sign_hmac(*hash, secret, &payload.data)?,
            _ => return Err("InvalidAccessError: key does not support sign".to_owned()),
        };

        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub fn subtle_verify(&self, payload: &str) -> Result<String, String> {
        let payload: VerifyPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        ensure_algorithm_name(&payload.algorithm.name, "HMAC")?;
        let record = self.get_key(&payload.key_id)?;

        if !record.usages.iter().any(|usage| usage == "verify") {
            return Err("InvalidAccessError: key does not allow verify".to_owned());
        }

        let verified = match &record.material {
            KeyMaterial::Hmac { secret, hash } => {
                verify_hmac(*hash, secret, &payload.data, &payload.signature)?
            }
            _ => return Err("InvalidAccessError: key does not support verify".to_owned()),
        };

        serde_json::to_string(&VerifyResultPayload { verified }).map_err(|error| error.to_string())
    }

    pub fn subtle_encrypt(&self, payload: &str) -> Result<String, String> {
        let payload: EncryptPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        ensure_algorithm_name(&payload.algorithm.name, "AES-GCM")?;
        let record = self.get_key(&payload.key_id)?;

        if !record.usages.iter().any(|usage| usage == "encrypt") {
            return Err("InvalidAccessError: key does not allow encrypt".to_owned());
        }

        let bytes = match &record.material {
            KeyMaterial::Aes { secret } => encrypt_aes_gcm(
                secret,
                payload
                    .algorithm
                    .iv
                    .as_deref()
                    .ok_or_else(|| "Missing algorithm.iv".to_owned())?,
                payload.algorithm.additional_data.as_deref().unwrap_or(&[]),
                &payload.data,
                payload.algorithm.tag_length,
            )?,
            _ => return Err("InvalidAccessError: key does not support encrypt".to_owned()),
        };

        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub fn subtle_decrypt(&self, payload: &str) -> Result<String, String> {
        let payload: DecryptPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        ensure_algorithm_name(&payload.algorithm.name, "AES-GCM")?;
        let record = self.get_key(&payload.key_id)?;

        if !record.usages.iter().any(|usage| usage == "decrypt") {
            return Err("InvalidAccessError: key does not allow decrypt".to_owned());
        }

        let bytes = match &record.material {
            KeyMaterial::Aes { secret } => ops::decrypt_aes_gcm(
                secret,
                payload
                    .algorithm
                    .iv
                    .as_deref()
                    .ok_or_else(|| "Missing algorithm.iv".to_owned())?,
                payload.algorithm.additional_data.as_deref().unwrap_or(&[]),
                &payload.data,
                payload.algorithm.tag_length,
            )?,
            _ => return Err("InvalidAccessError: key does not support decrypt".to_owned()),
        };

        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub fn subtle_derive_bits(&self, payload: &str) -> Result<String, String> {
        let payload: DeriveBitsPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        ensure_algorithm_name(&payload.algorithm.name, "PBKDF2")?;
        let record = self.get_key(&payload.key_id)?;

        if !record
            .usages
            .iter()
            .any(|usage| usage == "deriveBits" || usage == "deriveKey")
        {
            return Err("InvalidAccessError: key does not allow deriveBits".to_owned());
        }

        let bytes = match &record.material {
            KeyMaterial::Pbkdf2 { secret } => derive_pbkdf2_bits(
                secret,
                payload
                    .algorithm
                    .salt
                    .as_deref()
                    .ok_or_else(|| "Missing algorithm.salt".to_owned())?,
                payload
                    .algorithm
                    .iterations
                    .ok_or_else(|| "Missing algorithm.iterations".to_owned())?,
                parse_hash_from_descriptor(&payload.algorithm)?,
                payload.length,
            )?,
            _ => return Err("InvalidAccessError: key does not support deriveBits".to_owned()),
        };

        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    fn store_key(&self, record: CryptoKeyRecord) -> Result<String, String> {
        let id = format!("key-{}", self.next_key_id.fetch_add(1, Ordering::Relaxed));
        let descriptor = KeyDescriptor {
            id: id.clone(),
            key_type: record.key_type.clone(),
            extractable: record.extractable,
            algorithm: record.algorithm.clone(),
            usages: record.usages.clone(),
        };

        self.store
            .lock()
            .map_err(|_| "crypto key store poisoned".to_owned())?
            .insert(id, record);

        serde_json::to_string(&descriptor).map_err(|error| error.to_string())
    }

    fn get_key(&self, key_id: &str) -> Result<CryptoKeyRecord, String> {
        self.store
            .lock()
            .map_err(|_| "crypto key store poisoned".to_owned())?
            .get(key_id)
            .cloned()
            .ok_or_else(|| format!("Unknown CryptoKey id: {key_id}"))
    }
}

fn deserialize_raw_bytes(value: Value) -> Result<Vec<u8>, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}
