mod qrcode;

use chacha20poly1305::{
    aead::{
        stream::{DecryptorLE31, EncryptorLE31},
        OsRng,
    },
    ChaCha20Poly1305, KeyInit,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FileEncrypter {
    encryptor: Option<EncryptorLE31<ChaCha20Poly1305>>,
    key: [u8; 32],
    base_nonce: [u8; 8],
}

#[wasm_bindgen]
impl FileEncrypter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let key = ChaCha20Poly1305::generate_key(&mut OsRng);
        let cipher = ChaCha20Poly1305::new(&key);

        // EncryptorLE31 requires an 8-byte base nonce
        let mut base_nonce = [0u8; 8];
        getrandom::getrandom(&mut base_nonce).expect("Failed to get random bytes");

        let encryptor = EncryptorLE31::from_aead(cipher, &base_nonce.into());

        Self {
            encryptor: Some(encryptor),
            key: key.into(),
            base_nonce,
        }
    }

    pub fn export_key(&self) -> Vec<u8> {
        self.key.to_vec()
    }

    pub fn export_nonce(&self) -> Vec<u8> {
        self.base_nonce.to_vec()
    }

    pub fn encrypt_chunk(&mut self, data: &[u8], is_last: bool) -> Result<Vec<u8>, JsValue> {
        if is_last {
            let encryptor = self.encryptor.take().ok_or_else(|| JsValue::from_str("Stream already finished"))?;
            encryptor.encrypt_last(data).map_err(|_| JsValue::from_str("Encryption failed"))
        } else {
            let encryptor = self.encryptor.as_mut().ok_or_else(|| JsValue::from_str("Stream already finished"))?;
            encryptor.encrypt_next(data).map_err(|_| JsValue::from_str("Encryption failed"))
        }
    }
}

#[wasm_bindgen]
pub struct FileDecrypter {
    decryptor: Option<DecryptorLE31<ChaCha20Poly1305>>,
}

#[wasm_bindgen]
impl FileDecrypter {
    #[wasm_bindgen(constructor)]
    pub fn new(key_bytes: &[u8], nonce_bytes: &[u8]) -> Result<FileDecrypter, JsValue> {
        if key_bytes.len() != 32 || nonce_bytes.len() != 8 {
            return Err(JsValue::from_str("Invalid length. Key must be 32 bytes and nonce 8 bytes."));
        }

        let cipher = ChaCha20Poly1305::new(key_bytes.into());
        let mut nonce = [0u8; 8];
        nonce.copy_from_slice(nonce_bytes);
        let decryptor = DecryptorLE31::from_aead(cipher, &nonce.into());

        Ok(Self { decryptor: Some(decryptor) })
    }

    pub fn decrypt_chunk(&mut self, data: &[u8], is_last: bool) -> Result<Vec<u8>, JsValue> {
        if is_last {
            // Consumes the decryptor so it can't be used again
            let decryptor = self.decryptor.take().ok_or_else(|| JsValue::from_str("Stream already finished"))?;
            decryptor.decrypt_last(data).map_err(|_| JsValue::from_str("Decryption failed. The data may be corrupted or this is the wrong key."))
        } else {
            // Uses a mutable reference for continuous chunks
            let decryptor = self.decryptor.as_mut().ok_or_else(|| JsValue::from_str("Stream already finished"))?;
            decryptor.decrypt_next(data).map_err(|_| JsValue::from_str("Decryption failed. The data may be corrupted or this is the wrong key."))
        }
    }
}