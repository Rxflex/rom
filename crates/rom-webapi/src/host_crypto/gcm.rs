use aes::{
    Aes128, Aes192, Aes256,
    cipher::{
        BlockCipher, BlockEncrypt, BlockSizeUser, KeyInit,
        generic_array::{
            GenericArray,
            typenum::{U12, U13, U14, U15, U16},
        },
    },
};
use aes_gcm::{
    AesGcm,
    aead::AeadInPlace,
};

pub(crate) fn encrypt_aes_gcm(
    secret: &[u8],
    iv: &[u8],
    additional_data: &[u8],
    data: &[u8],
    tag_length: Option<usize>,
) -> Result<Vec<u8>, String> {
    let nonce = prepare_nonce(iv)?;
    match (secret.len(), normalize_tag_length(tag_length)?) {
        (16, 96) => encrypt_aes_gcm_with_tag::<Aes128, U12>(secret, nonce, additional_data, data),
        (16, 104) => {
            encrypt_aes_gcm_with_tag::<Aes128, U13>(secret, nonce, additional_data, data)
        }
        (16, 112) => {
            encrypt_aes_gcm_with_tag::<Aes128, U14>(secret, nonce, additional_data, data)
        }
        (16, 120) => {
            encrypt_aes_gcm_with_tag::<Aes128, U15>(secret, nonce, additional_data, data)
        }
        (16, 128) => {
            encrypt_aes_gcm_with_tag::<Aes128, U16>(secret, nonce, additional_data, data)
        }
        (24, 96) => encrypt_aes_gcm_with_tag::<Aes192, U12>(secret, nonce, additional_data, data),
        (24, 104) => {
            encrypt_aes_gcm_with_tag::<Aes192, U13>(secret, nonce, additional_data, data)
        }
        (24, 112) => {
            encrypt_aes_gcm_with_tag::<Aes192, U14>(secret, nonce, additional_data, data)
        }
        (24, 120) => {
            encrypt_aes_gcm_with_tag::<Aes192, U15>(secret, nonce, additional_data, data)
        }
        (24, 128) => {
            encrypt_aes_gcm_with_tag::<Aes192, U16>(secret, nonce, additional_data, data)
        }
        (32, 96) => encrypt_aes_gcm_with_tag::<Aes256, U12>(secret, nonce, additional_data, data),
        (32, 104) => {
            encrypt_aes_gcm_with_tag::<Aes256, U13>(secret, nonce, additional_data, data)
        }
        (32, 112) => {
            encrypt_aes_gcm_with_tag::<Aes256, U14>(secret, nonce, additional_data, data)
        }
        (32, 120) => {
            encrypt_aes_gcm_with_tag::<Aes256, U15>(secret, nonce, additional_data, data)
        }
        (32, 128) => {
            encrypt_aes_gcm_with_tag::<Aes256, U16>(secret, nonce, additional_data, data)
        }
        (16 | 24 | 32, other) => Err(format!("Unsupported AES-GCM tagLength: {other}")),
        (other, _) => Err(format!(
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
    match (secret.len(), normalize_tag_length(tag_length)?) {
        (16, 96) => decrypt_aes_gcm_with_tag::<Aes128, U12>(secret, nonce, additional_data, data),
        (16, 104) => {
            decrypt_aes_gcm_with_tag::<Aes128, U13>(secret, nonce, additional_data, data)
        }
        (16, 112) => {
            decrypt_aes_gcm_with_tag::<Aes128, U14>(secret, nonce, additional_data, data)
        }
        (16, 120) => {
            decrypt_aes_gcm_with_tag::<Aes128, U15>(secret, nonce, additional_data, data)
        }
        (16, 128) => {
            decrypt_aes_gcm_with_tag::<Aes128, U16>(secret, nonce, additional_data, data)
        }
        (24, 96) => decrypt_aes_gcm_with_tag::<Aes192, U12>(secret, nonce, additional_data, data),
        (24, 104) => {
            decrypt_aes_gcm_with_tag::<Aes192, U13>(secret, nonce, additional_data, data)
        }
        (24, 112) => {
            decrypt_aes_gcm_with_tag::<Aes192, U14>(secret, nonce, additional_data, data)
        }
        (24, 120) => {
            decrypt_aes_gcm_with_tag::<Aes192, U15>(secret, nonce, additional_data, data)
        }
        (24, 128) => {
            decrypt_aes_gcm_with_tag::<Aes192, U16>(secret, nonce, additional_data, data)
        }
        (32, 96) => decrypt_aes_gcm_with_tag::<Aes256, U12>(secret, nonce, additional_data, data),
        (32, 104) => {
            decrypt_aes_gcm_with_tag::<Aes256, U13>(secret, nonce, additional_data, data)
        }
        (32, 112) => {
            decrypt_aes_gcm_with_tag::<Aes256, U14>(secret, nonce, additional_data, data)
        }
        (32, 120) => {
            decrypt_aes_gcm_with_tag::<Aes256, U15>(secret, nonce, additional_data, data)
        }
        (32, 128) => {
            decrypt_aes_gcm_with_tag::<Aes256, U16>(secret, nonce, additional_data, data)
        }
        (16 | 24 | 32, other) => Err(format!("Unsupported AES-GCM tagLength: {other}")),
        (other, _) => Err(format!(
            "Unsupported AES-GCM raw key length: {} bits",
            other * 8
        )),
    }
}

fn encrypt_aes_gcm_with_tag<Aes, TagSize>(
    secret: &[u8],
    nonce: &GenericArray<u8, U12>,
    additional_data: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, String>
where
    Aes: BlockCipher + BlockEncrypt + BlockSizeUser<BlockSize = U16> + KeyInit,
    TagSize: aes_gcm::TagSize,
{
    let cipher = AesGcm::<Aes, U12, TagSize>::new_from_slice(secret)
        .map_err(|error| error.to_string())?;
    let mut buffer = data.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(nonce, additional_data, &mut buffer)
        .map_err(|_| "OperationError: AES-GCM encryption failed".to_owned())?;
    buffer.extend_from_slice(tag.as_slice());
    Ok(buffer)
}

fn decrypt_aes_gcm_with_tag<Aes, TagSize>(
    secret: &[u8],
    nonce: &GenericArray<u8, U12>,
    additional_data: &[u8],
    data: &[u8],
) -> Result<Vec<u8>, String>
where
    Aes: BlockCipher + BlockEncrypt + BlockSizeUser<BlockSize = U16> + KeyInit,
    TagSize: aes_gcm::TagSize,
{
    let tag_len = TagSize::to_usize();
    let cipher = AesGcm::<Aes, U12, TagSize>::new_from_slice(secret)
        .map_err(|error| error.to_string())?;
    let split = data
        .len()
        .checked_sub(tag_len)
        .ok_or_else(|| "OperationError: AES-GCM ciphertext shorter than tag".to_owned())?;
    let (ciphertext, tag_bytes) = data.split_at(split);
    let mut buffer = ciphertext.to_vec();
    let tag = GenericArray::<u8, TagSize>::clone_from_slice(tag_bytes);
    cipher
        .decrypt_in_place_detached(nonce, additional_data, &mut buffer, &tag)
        .map_err(|_| "OperationError: AES-GCM decryption failed".to_owned())?;
    Ok(buffer)
}

fn normalize_tag_length(tag_length: Option<usize>) -> Result<usize, String> {
    match tag_length.unwrap_or(128) {
        96 | 104 | 112 | 120 | 128 => Ok(tag_length.unwrap_or(128)),
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
