use serde_json::Value;

use super::*;

impl CryptoHost {
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
                    | KeyMaterial::Pbkdf2 { secret }
                    | KeyMaterial::Hkdf { secret } => secret.clone(),
                };
                serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
            }
            "jwk" => {
                let exported = match &record.material {
                    KeyMaterial::Hmac { secret, hash } => serde_json::to_value(export_hmac_jwk(
                        secret.as_slice(),
                        hash.to_owned(),
                        record.extractable,
                        record.usages.clone(),
                    ))
                    .map_err(|error| error.to_string())?,
                    KeyMaterial::Aes { secret } => serde_json::to_value(export_aes_jwk(
                        secret.as_slice(),
                        &record.algorithm.name,
                        record.extractable,
                        record.usages.clone(),
                    )?)
                    .map_err(|error| error.to_string())?,
                    KeyMaterial::Pbkdf2 { .. } => {
                        return Err("Unsupported key export format for PBKDF2: jwk".to_owned());
                    }
                    KeyMaterial::Hkdf { .. } => {
                        return Err("Unsupported key export format for HKDF: jwk".to_owned());
                    }
                };
                serde_json::to_string(&exported).map_err(|error| error.to_string())
            }
            other => Err(format!("Unsupported key export format: {other}")),
        }
    }

    pub fn subtle_digest(&self, payload: &str) -> Result<String, String> {
        let payload: DigestPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let algorithm = parse_hash_algorithm(&payload.algorithm)?;
        let bytes = digest_bytes(algorithm, &payload.data);
        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
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

    pub fn subtle_derive_bits(&self, payload: &str) -> Result<String, String> {
        let payload: DeriveBitsPayload =
            serde_json::from_str(payload).map_err(|error| error.to_string())?;
        let record = self.get_key(&payload.key_id)?;

        if !record
            .usages
            .iter()
            .any(|usage| usage == "deriveBits" || usage == "deriveKey")
        {
            return Err("InvalidAccessError: key does not allow deriveBits".to_owned());
        }

        let bytes = match payload.algorithm.name.to_ascii_uppercase().as_str() {
            "PBKDF2" => match &record.material {
                KeyMaterial::Pbkdf2 { secret } => derive_pbkdf2_bits(
                    secret.as_slice(),
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
            },
            "HKDF" => match &record.material {
                KeyMaterial::Hkdf { secret } => derive_hkdf_bits(
                    secret.as_slice(),
                    payload
                        .algorithm
                        .salt
                        .as_deref()
                        .ok_or_else(|| "Missing algorithm.salt".to_owned())?,
                    payload.algorithm.info.as_deref().unwrap_or(&[]),
                    parse_hash_from_descriptor(&payload.algorithm)?,
                    payload.length,
                )?,
                _ => return Err("InvalidAccessError: key does not support deriveBits".to_owned()),
            },
            other => return Err(format!("Unsupported algorithm: {other}")),
        };

        serde_json::to_string(&BytesPayload { bytes }).map_err(|error| error.to_string())
    }

    pub(super) fn store_key(&self, record: CryptoKeyRecord) -> Result<String, String> {
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

    pub(super) fn get_key(&self, key_id: &str) -> Result<CryptoKeyRecord, String> {
        self.store
            .lock()
            .map_err(|_| "crypto key store poisoned".to_owned())?
            .get(key_id)
            .cloned()
            .ok_or_else(|| format!("Unknown CryptoKey id: {key_id}"))
    }
}

pub(super) fn deserialize_raw_bytes(value: Value) -> Result<Vec<u8>, String> {
    serde_json::from_value(value).map_err(|error| error.to_string())
}
