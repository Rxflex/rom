use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct CryptoKeyRecord {
    pub extractable: bool,
    pub key_type: String,
    pub algorithm: KeyAlgorithm,
    pub usages: Vec<String>,
    pub material: KeyMaterial,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KeyAlgorithm {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<KeyHashAlgorithm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct KeyHashAlgorithm {
    pub name: String,
}

#[derive(Debug, Clone)]
pub(crate) enum KeyMaterial {
    Hmac {
        secret: Vec<u8>,
        hash: HashAlgorithm,
    },
    Aes {
        secret: Vec<u8>,
    },
    Pbkdf2 {
        secret: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum HashAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

#[derive(Debug, Deserialize)]
pub struct DigestPayload {
    pub algorithm: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateKeyPayload {
    pub algorithm: AlgorithmDescriptor,
    pub extractable: bool,
    pub usages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportKeyPayload {
    pub format: String,
    pub key_data: serde_json::Value,
    pub algorithm: AlgorithmDescriptor,
    pub extractable: bool,
    pub usages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportKeyPayload {
    pub format: String,
    pub key_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SignPayload {
    pub algorithm: AlgorithmDescriptor,
    pub key_id: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPayload {
    pub algorithm: AlgorithmDescriptor,
    pub key_id: String,
    pub signature: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct EncryptPayload {
    pub algorithm: AlgorithmDescriptor,
    pub key_id: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct DecryptPayload {
    pub algorithm: AlgorithmDescriptor,
    pub key_id: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct DeriveBitsPayload {
    pub algorithm: AlgorithmDescriptor,
    pub key_id: String,
    pub length: usize,
}

#[derive(Debug, Deserialize)]
pub struct AlgorithmDescriptor {
    pub name: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub length: Option<usize>,
    #[serde(default)]
    pub iv: Option<Vec<u8>>,
    #[serde(default)]
    pub additional_data: Option<Vec<u8>>,
    #[serde(default)]
    pub tag_length: Option<usize>,
    #[serde(default)]
    pub salt: Option<Vec<u8>>,
    #[serde(default)]
    pub iterations: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JsonWebKey {
    pub kty: String,
    pub k: String,
    #[serde(default)]
    pub alg: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BytesPayload {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub(crate) struct VerifyResultPayload {
    pub verified: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct KeyDescriptor {
    pub id: String,
    #[serde(rename = "type")]
    pub key_type: String,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub usages: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ExportedJwk {
    pub kty: &'static str,
    pub k: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alg: Option<&'static str>,
    pub ext: bool,
    pub key_ops: Vec<String>,
}

impl HashAlgorithm {
    pub(crate) fn web_name(self) -> &'static str {
        match self {
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha384 => "SHA-384",
            HashAlgorithm::Sha512 => "SHA-512",
        }
    }

    pub(crate) fn jwk_alg_name(self) -> &'static str {
        match self {
            HashAlgorithm::Sha1 => "HS1",
            HashAlgorithm::Sha256 => "HS256",
            HashAlgorithm::Sha384 => "HS384",
            HashAlgorithm::Sha512 => "HS512",
        }
    }
}
