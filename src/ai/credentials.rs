//! Authenticated encryption for personal provider keys.
//!
//! Keys are sealed with AES-256-GCM using a 32-byte key supplied by the
//! deployment (`AI_ENCRYPTION_KEY`, base64). The stored envelope is
//! `v1:<base64url(nonce || ciphertext || tag)>`; neither the plaintext nor the
//! encryption key is ever written to the database or returned by an endpoint.

use std::fmt;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};

use crate::ai::error::AiError;

const PREFIX: &str = "v1:";
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// A configured credential cipher. Cloning shares no plaintext.
#[derive(Clone)]
pub struct SecretBox {
    cipher: Aes256Gcm,
}

impl fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretBox(<redacted>)")
    }
}

impl SecretBox {
    /// Build a cipher from a base64-encoded 32-byte key.
    ///
    /// # Errors
    ///
    /// Returns an error when the value is not valid base64 or is not exactly
    /// 32 bytes.
    pub fn from_base64(value: &str) -> Result<Option<Self>, AiError> {
        let value = value.trim();
        if value.is_empty() {
            return Ok(None);
        }

        let bytes = STANDARD
            .decode(value)
            .or_else(|_| URL_SAFE_NO_PAD.decode(value))
            .map_err(|_| {
                AiError::Encryption("AI_ENCRYPTION_KEY must be base64-encoded".to_string())
            })?;
        if bytes.len() != KEY_LEN {
            return Err(AiError::Encryption(
                "AI_ENCRYPTION_KEY must decode to exactly 32 bytes".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&bytes)
            .map_err(|_| AiError::Encryption("AI_ENCRYPTION_KEY is not a valid key".to_string()))?;
        Ok(Some(Self { cipher }))
    }

    /// Seal a plaintext credential.
    ///
    /// # Errors
    ///
    /// Returns an error if the cipher cannot encrypt the value.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, AiError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| AiError::Encryption("could not encrypt the credential".to_string()))?;

        let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        packed.extend_from_slice(&nonce);
        packed.extend_from_slice(&ciphertext);
        Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(packed)))
    }

    /// Open a stored credential envelope.
    ///
    /// # Errors
    ///
    /// Returns an error when the envelope is malformed, was sealed with another
    /// key, or does not contain UTF-8.
    pub fn decrypt(&self, envelope: &str) -> Result<String, AiError> {
        let encoded = envelope.strip_prefix(PREFIX).ok_or_else(|| {
            AiError::Encryption("the stored credential has an unknown format".to_string())
        })?;
        let packed = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
            AiError::Encryption("the stored credential is not valid base64".to_string())
        })?;
        if packed.len() <= NONCE_LEN {
            return Err(AiError::Encryption(
                "the stored credential is truncated".to_string(),
            ));
        }

        let (nonce, ciphertext) = packed.split_at(NONCE_LEN);
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| {
                AiError::Encryption("the stored credential could not be decrypted".to_string())
            })?;
        String::from_utf8(plaintext).map_err(|_| {
            AiError::Encryption("the stored credential is not valid UTF-8".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> String {
        STANDARD.encode([7u8; KEY_LEN])
    }

    #[test]
    fn round_trips_a_credential() {
        let secret = SecretBox::from_base64(&key()).unwrap().unwrap();
        let envelope = secret.encrypt("sk-or-v1-example").unwrap();

        assert!(envelope.starts_with(PREFIX));
        assert!(!envelope.contains("sk-or-v1-example"));
        assert_eq!(secret.decrypt(&envelope).unwrap(), "sk-or-v1-example");
    }

    #[test]
    fn a_different_key_cannot_open_the_envelope() {
        let secret = SecretBox::from_base64(&key()).unwrap().unwrap();
        let other = SecretBox::from_base64(&STANDARD.encode([8u8; KEY_LEN]))
            .unwrap()
            .unwrap();
        let envelope = secret.encrypt("sk-or-v1-example").unwrap();

        assert!(other.decrypt(&envelope).is_err());
    }

    #[test]
    fn rejects_malformed_keys_and_envelopes() {
        assert!(SecretBox::from_base64("not base64").is_err());
        assert!(SecretBox::from_base64(&STANDARD.encode([1u8; 16])).is_err());
        assert!(SecretBox::from_base64("").unwrap().is_none());

        let secret = SecretBox::from_base64(&key()).unwrap().unwrap();
        assert!(secret.decrypt("plaintext").is_err());
        assert!(secret.decrypt("v1:not-base64").is_err());
    }

    #[test]
    fn debug_output_does_not_reveal_the_key() {
        let secret = SecretBox::from_base64(&key()).unwrap().unwrap();
        assert_eq!(format!("{secret:?}"), "SecretBox(<redacted>)");
    }
}
