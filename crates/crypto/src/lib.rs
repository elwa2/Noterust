//! Password based encryption for locked notes.
//!
//! * Key derivation: Argon2id (default parameters from the `argon2` crate).
//! * Encryption: AES-256-GCM with a random 96-bit nonce.
//! * Blob layout: `enc:v1:` + base64(`salt(16) || nonce(12) || ciphertext+tag`).
//!
//! Every encryption uses a fresh salt and nonce, so the same note encrypted
//! twice produces different blobs.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rand::RngCore;
use zeroize::Zeroize;

/// Prefix that marks a stored blob as encrypted.
pub const PREFIX: &str = "enc:v1:";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// Errors returned by the crypto layer.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("كلمة المرور غير صحيحة أو البيانات تالفة")]
    InvalidPassword,
    #[error("البيانات المشفرة غير صالحة")]
    Malformed,
    #[error("تعذر اشتقاق المفتاح: {0}")]
    Kdf(String),
    #[error("فشل التشفير")]
    Encrypt,
    #[error("خطأ في فك الترميز")]
    Base64,
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|err| CryptoError::Kdf(err.to_string()))?;
    Ok(key)
}

/// Encrypts `plaintext` with `password`, returning an `enc:v1:` blob.
pub fn encrypt(plaintext: &str, password: &str) -> Result<String, CryptoError> {
    if password.is_empty() {
        return Err(CryptoError::Kdf("كلمة المرور فارغة".into()));
    }
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    let mut key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad: PREFIX.as_bytes(),
            },
        )
        .map_err(|_| CryptoError::Encrypt)?;
    key.zeroize();

    let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(format!("{PREFIX}{}", BASE64.encode(blob)))
}

/// Decrypts a blob produced by [`encrypt`].
pub fn decrypt(blob: &str, password: &str) -> Result<String, CryptoError> {
    let payload = blob.strip_prefix(PREFIX).ok_or(CryptoError::Malformed)?;
    let raw = BASE64.decode(payload).map_err(|_| CryptoError::Base64)?;
    if raw.len() <= SALT_LEN + NONCE_LEN {
        return Err(CryptoError::Malformed);
    }
    let (salt, rest) = raw.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    let mut key = derive_key(password, salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad: PREFIX.as_bytes(),
            },
        )
        .map_err(|_| CryptoError::InvalidPassword)?;
    key.zeroize();

    String::from_utf8(plaintext).map_err(|_| CryptoError::Malformed)
}

/// True when the value stored for a note is an encrypted blob.
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_returns_original() {
        let blob = encrypt("ملاحظة سرية", "كلمة-مرور-قوية").expect("encrypt");
        assert!(is_encrypted(&blob));
        assert_eq!(
            decrypt(&blob, "كلمة-مرور-قوية").expect("decrypt"),
            "ملاحظة سرية"
        );
    }

    #[test]
    fn wrong_password_is_rejected() {
        let blob = encrypt("secret", "correct").expect("encrypt");
        assert!(matches!(
            decrypt(&blob, "wrong"),
            Err(CryptoError::InvalidPassword)
        ));
    }

    #[test]
    fn ciphertext_is_not_plaintext() {
        let blob = encrypt("secret", "pass").expect("encrypt");
        assert!(!blob.contains("secret"));
    }

    #[test]
    fn fresh_salt_gives_different_blobs() {
        let first = encrypt("same", "pass").expect("encrypt");
        let second = encrypt("same", "pass").expect("encrypt");
        assert_ne!(first, second);
    }

    #[test]
    fn malformed_blob_is_rejected() {
        assert!(matches!(
            decrypt("not-encrypted", "pass"),
            Err(CryptoError::Malformed)
        ));
        assert!(matches!(
            decrypt("enc:v1:@@@", "pass"),
            Err(CryptoError::Base64)
        ));
    }

    #[test]
    fn empty_password_is_rejected() {
        assert!(encrypt("x", "").is_err());
    }
}
