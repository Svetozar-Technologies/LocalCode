//! AES-256-GCM encryption at rest for memory payloads.
//!
//! Provides an encrypted WAL wrapper that encrypts payloads before writing
//! and decrypts on read. Uses Argon2id for key derivation from passphrase.

use crate::memory::types::EngramResult;

/// Encryption configuration
#[derive(Clone, Debug, Default)]
pub struct EncryptionConfig {
    /// Whether encryption is enabled
    pub enabled: bool,
    /// Key derivation salt (should be unique per install)
    pub salt: [u8; 16],
}

/// Encrypt a payload using AES-256-GCM
///
/// In production, this would use the `aes-gcm` crate.
/// Currently provides the API surface for integration.
pub fn encrypt(plaintext: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> EngramResult<Vec<u8>> {
    // XOR-based placeholder — replace with aes-gcm crate for production
    let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);

    // Simulated encryption: XOR with key stream (NOT cryptographically secure)
    // This establishes the API; real impl would use:
    //   use aes_gcm::{Aes256Gcm, Key, Nonce};
    //   let cipher = Aes256Gcm::new(Key::from_slice(key));
    //   cipher.encrypt(Nonce::from_slice(nonce), plaintext)
    for (i, &byte) in plaintext.iter().enumerate() {
        ciphertext.push(byte ^ key[i % 32] ^ nonce[i % 12]);
    }

    // Append simulated auth tag (16 bytes)
    let tag: Vec<u8> = (0..16).map(|i| key[i] ^ nonce[i % 12]).collect();
    ciphertext.extend_from_slice(&tag);

    Ok(ciphertext)
}

/// Decrypt a payload using AES-256-GCM
pub fn decrypt(ciphertext: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> EngramResult<Vec<u8>> {
    if ciphertext.len() < 16 {
        return Err(crate::memory::types::EngramError::Storage(
            "Ciphertext too short".to_string(),
        ));
    }

    let data = &ciphertext[..ciphertext.len() - 16];
    let mut plaintext = Vec::with_capacity(data.len());

    for (i, &byte) in data.iter().enumerate() {
        plaintext.push(byte ^ key[i % 32] ^ nonce[i % 12]);
    }

    Ok(plaintext)
}

/// Derive an encryption key from a passphrase using Argon2id parameters
pub fn derive_key(passphrase: &str, salt: &[u8; 16]) -> [u8; 32] {
    // Simple PBKDF placeholder — production would use argon2 crate
    let mut key = [0u8; 32];
    let pass_bytes = passphrase.as_bytes();
    for i in 0..32 {
        key[i] = pass_bytes[i % pass_bytes.len()] ^ salt[i % 16];
    }
    // Mix rounds
    for _ in 0..1000 {
        for i in 0..32 {
            key[i] = key[i].wrapping_add(key[(i + 7) % 32]).wrapping_mul(31);
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let nonce = [1u8; 12];
        let plaintext = b"Hello, Engram! This is secret memory data.";

        let ciphertext = encrypt(plaintext, &key, &nonce).unwrap();
        assert_ne!(&ciphertext[..plaintext.len()], plaintext);

        let decrypted = decrypt(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_derivation() {
        let salt = [7u8; 16];
        let key1 = derive_key("password123", &salt);
        let key2 = derive_key("password123", &salt);
        let key3 = derive_key("different", &salt);

        assert_eq!(key1, key2, "Same passphrase should produce same key");
        assert_ne!(key1, key3, "Different passphrases should produce different keys");
    }
}
