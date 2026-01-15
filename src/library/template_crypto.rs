//! Cryptographic utilities for workspace template environment variables.
//!
//! Provides transparent encryption/decryption of env var values using AES-256-GCM.
//! Encrypted values are wrapped in `<encrypted v="1">NONCE:CIPHERTEXT</encrypted>`.
//!
//! Key management:
//! - Key is loaded from `TEMPLATE_ENCRYPTION_KEY` env var (32-byte hex or base64)
//! - If missing at startup, a key is generated and appended to `.env`
//! - If key generation fails, encryption is disabled (values stored as plaintext)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use thiserror::Error;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

/// Environment variable name for the encryption key.
pub const ENCRYPTION_KEY_ENV: &str = "TEMPLATE_ENCRYPTION_KEY";

/// Prefix for encrypted values.
const ENCRYPTED_PREFIX: &str = "<encrypted v=\"1\">";
/// Suffix for encrypted values.
const ENCRYPTED_SUFFIX: &str = "</encrypted>";

/// Nonce length in bytes (96 bits for AES-GCM).
const NONCE_LENGTH: usize = 12;

/// Key length in bytes (256 bits for AES-256).
const KEY_LENGTH: usize = 32;

/// Global encryption key, loaded once at startup.
static ENCRYPTION_KEY: OnceLock<Option<[u8; KEY_LENGTH]>> = OnceLock::new();

/// Errors that can occur during template crypto operations.
#[derive(Debug, Error)]
pub enum TemplateCryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid encrypted format: {0}")]
    InvalidFormat(String),

    #[error("Invalid base64: {0}")]
    InvalidBase64(String),

    #[error("Encryption key not available")]
    KeyNotAvailable,
}

/// Initialize the encryption key from environment or generate a new one.
///
/// This should be called once at application startup. If `TEMPLATE_ENCRYPTION_KEY`
/// is not set, a new key will be generated and appended to the `.env` file.
///
/// # Arguments
/// * `env_file_path` - Path to the `.env` file for key persistence
///
/// # Returns
/// * `true` if encryption is available (key loaded or generated)
/// * `false` if encryption is disabled (no key and couldn't generate)
pub async fn init_encryption_key(env_file_path: Option<&Path>) -> bool {
    // Check if already initialized
    if ENCRYPTION_KEY.get().is_some() {
        return ENCRYPTION_KEY.get().unwrap().is_some();
    }

    let key = match load_key_from_env() {
        Some(key) => {
            info!("Template encryption key loaded from environment");
            Some(key)
        }
        None => {
            // Try to generate a new key
            if let Some(path) = env_file_path {
                match generate_and_persist_key(path).await {
                    Ok(key) => {
                        info!("Generated new template encryption key and saved to .env");
                        Some(key)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to generate encryption key, template env vars will be stored as plaintext: {}",
                            e
                        );
                        None
                    }
                }
            } else {
                warn!(
                    "TEMPLATE_ENCRYPTION_KEY not set and no .env path provided, \
                     template env vars will be stored as plaintext"
                );
                None
            }
        }
    };

    let available = key.is_some();
    let _ = ENCRYPTION_KEY.set(key);
    available
}

/// Get the encryption key if available.
fn get_key() -> Option<&'static [u8; KEY_LENGTH]> {
    ENCRYPTION_KEY.get().and_then(|k| k.as_ref())
}

/// Load the encryption key from the environment variable.
fn load_key_from_env() -> Option<[u8; KEY_LENGTH]> {
    let key_str = std::env::var(ENCRYPTION_KEY_ENV).ok()?;
    let key_str = key_str.trim();

    if key_str.is_empty() {
        return None;
    }

    // Try hex first (64 characters for 32 bytes)
    if key_str.len() == 64 && key_str.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(bytes) = hex::decode(key_str) {
            if bytes.len() == KEY_LENGTH {
                let mut key = [0u8; KEY_LENGTH];
                key.copy_from_slice(&bytes);
                return Some(key);
            }
        }
    }

    // Try base64
    if let Ok(bytes) = BASE64.decode(key_str) {
        if bytes.len() == KEY_LENGTH {
            let mut key = [0u8; KEY_LENGTH];
            key.copy_from_slice(&bytes);
            return Some(key);
        }
    }

    warn!(
        "TEMPLATE_ENCRYPTION_KEY is set but invalid (expected 32 bytes as hex or base64)"
    );
    None
}

/// Generate a new encryption key and append it to the .env file.
async fn generate_and_persist_key(env_path: &Path) -> Result<[u8; KEY_LENGTH], std::io::Error> {
    // Generate random key
    let mut key = [0u8; KEY_LENGTH];
    rand::thread_rng().fill_bytes(&mut key);

    // Encode as hex for .env (easier to read/copy)
    let key_hex = hex::encode(key);

    // Append to .env file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_path)
        .await?;

    let content = format!(
        "\n# Template encryption key (auto-generated, DO NOT COMMIT)\n{}={}\n",
        ENCRYPTION_KEY_ENV, key_hex
    );

    file.write_all(content.as_bytes()).await?;
    file.flush().await?;

    // Also set in current process environment
    std::env::set_var(ENCRYPTION_KEY_ENV, &key_hex);

    Ok(key)
}

/// Check if a string value is encrypted (wrapped in `<encrypted v="1">...</encrypted>`).
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENCRYPTED_PREFIX) && value.ends_with(ENCRYPTED_SUFFIX)
}

/// Encrypt a plaintext string.
///
/// Returns the encrypted value wrapped in `<encrypted v="1">NONCE:CIPHERTEXT</encrypted>`.
/// If encryption is not available or the value is already encrypted, returns the original.
///
/// # Arguments
/// * `plaintext` - The value to encrypt
///
/// # Returns
/// * Encrypted string if successful
/// * Original string if encryption is disabled or value is already encrypted
pub fn encrypt_string(plaintext: &str) -> Result<String, TemplateCryptoError> {
    // Don't double-encrypt
    if is_encrypted(plaintext) {
        return Ok(plaintext.to_string());
    }

    let key = get_key().ok_or(TemplateCryptoError::KeyNotAvailable)?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_LENGTH];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    // Create cipher and encrypt
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| TemplateCryptoError::EncryptionFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| TemplateCryptoError::EncryptionFailed(e.to_string()))?;

    // Encode as base64 and wrap
    let nonce_b64 = BASE64.encode(nonce_bytes);
    let ciphertext_b64 = BASE64.encode(&ciphertext);

    Ok(format!(
        "{}{}:{}{}",
        ENCRYPTED_PREFIX, nonce_b64, ciphertext_b64, ENCRYPTED_SUFFIX
    ))
}

/// Decrypt a string value.
///
/// If the value is not encrypted (no `<encrypted>` wrapper), returns it unchanged.
/// This provides backward compatibility with plaintext values.
///
/// # Arguments
/// * `value` - The value to decrypt (may be plaintext or encrypted)
///
/// # Returns
/// * Decrypted plaintext if encrypted
/// * Original value if plaintext
/// * Error if decryption fails
pub fn decrypt_string(value: &str) -> Result<String, TemplateCryptoError> {
    // Pass through plaintext values unchanged (backward compatibility)
    if !is_encrypted(value) {
        return Ok(value.to_string());
    }

    let key = get_key().ok_or(TemplateCryptoError::KeyNotAvailable)?;

    // Extract the inner content
    let inner = value
        .strip_prefix(ENCRYPTED_PREFIX)
        .and_then(|s| s.strip_suffix(ENCRYPTED_SUFFIX))
        .ok_or_else(|| TemplateCryptoError::InvalidFormat("malformed encrypted tag".to_string()))?;

    // Split nonce and ciphertext
    let parts: Vec<&str> = inner.split(':').collect();
    if parts.len() != 2 {
        return Err(TemplateCryptoError::InvalidFormat(
            "expected NONCE:CIPHERTEXT format".to_string(),
        ));
    }

    let nonce_bytes = BASE64
        .decode(parts[0])
        .map_err(|e| TemplateCryptoError::InvalidBase64(e.to_string()))?;

    let ciphertext = BASE64
        .decode(parts[1])
        .map_err(|e| TemplateCryptoError::InvalidBase64(e.to_string()))?;

    if nonce_bytes.len() != NONCE_LENGTH {
        return Err(TemplateCryptoError::InvalidFormat(format!(
            "nonce length {} != {}",
            nonce_bytes.len(),
            NONCE_LENGTH
        )));
    }

    // Decrypt
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| TemplateCryptoError::DecryptionFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| TemplateCryptoError::DecryptionFailed("decryption failed".to_string()))?;

    String::from_utf8(plaintext)
        .map_err(|e| TemplateCryptoError::DecryptionFailed(format!("invalid UTF-8: {}", e)))
}

/// Encrypt all values in an env_vars HashMap.
///
/// Values that are already encrypted are left unchanged.
/// If encryption is not available, returns the original HashMap unchanged.
pub fn encrypt_env_vars(env_vars: &HashMap<String, String>) -> HashMap<String, String> {
    if get_key().is_none() {
        // Encryption disabled, return as-is
        return env_vars.clone();
    }

    env_vars
        .iter()
        .map(|(k, v)| {
            let encrypted = encrypt_string(v).unwrap_or_else(|_| v.clone());
            (k.clone(), encrypted)
        })
        .collect()
}

/// Decrypt all values in an env_vars HashMap.
///
/// Plaintext values are left unchanged (backward compatibility).
/// If a value fails to decrypt, logs a warning and returns the original value.
pub fn decrypt_env_vars(env_vars: &HashMap<String, String>) -> HashMap<String, String> {
    env_vars
        .iter()
        .map(|(k, v)| {
            let decrypted = decrypt_string(v).unwrap_or_else(|e| {
                if is_encrypted(v) {
                    warn!("Failed to decrypt env var '{}': {}", k, e);
                }
                v.clone()
            });
            (k.clone(), decrypted)
        })
        .collect()
}

/// Check if encryption is currently available.
pub fn is_encryption_available() -> bool {
    get_key().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set up a test key for use in tests.
    fn setup_test_key() {
        // Generate a deterministic test key
        let key = [0x42u8; KEY_LENGTH];
        let _ = ENCRYPTION_KEY.set(Some(key));
    }

    #[test]
    fn test_is_encrypted() {
        assert!(is_encrypted("<encrypted v=\"1\">abc:def</encrypted>"));
        assert!(!is_encrypted("plaintext"));
        assert!(!is_encrypted("<encrypted v=\"1\">incomplete"));
        assert!(!is_encrypted("</encrypted>"));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Reset the key for this test
        setup_test_key();

        let plaintext = "my-secret-api-key-12345";
        let encrypted = encrypt_string(plaintext).unwrap();

        // Should be wrapped
        assert!(is_encrypted(&encrypted));
        assert!(encrypted.starts_with(ENCRYPTED_PREFIX));
        assert!(encrypted.ends_with(ENCRYPTED_SUFFIX));

        // Should decrypt back to original
        let decrypted = decrypt_string(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_no_double_encrypt() {
        setup_test_key();

        let plaintext = "secret";
        let encrypted = encrypt_string(plaintext).unwrap();

        // Encrypting again should return the same value
        let double_encrypted = encrypt_string(&encrypted).unwrap();
        assert_eq!(encrypted, double_encrypted);
    }

    #[test]
    fn test_plaintext_passthrough() {
        setup_test_key();

        let plaintext = "not-encrypted-value";
        let result = decrypt_string(plaintext).unwrap();
        assert_eq!(result, plaintext);
    }

    #[test]
    fn test_different_encryptions_produce_different_ciphertext() {
        setup_test_key();

        let plaintext = "same-data";
        let encrypted1 = encrypt_string(plaintext).unwrap();
        let encrypted2 = encrypt_string(plaintext).unwrap();

        // Different nonces should produce different ciphertext
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(decrypt_string(&encrypted1).unwrap(), plaintext);
        assert_eq!(decrypt_string(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_encrypt_decrypt_env_vars() {
        setup_test_key();

        let mut original = HashMap::new();
        original.insert("API_KEY".to_string(), "sk-12345".to_string());
        original.insert("DATABASE_URL".to_string(), "postgres://...".to_string());

        let encrypted = encrypt_env_vars(&original);

        // All values should be encrypted
        for value in encrypted.values() {
            assert!(is_encrypted(value));
        }

        // Decrypting should restore originals
        let decrypted = decrypt_env_vars(&encrypted);
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_mixed_encrypted_plaintext() {
        setup_test_key();

        let mut mixed = HashMap::new();
        mixed.insert("ENCRYPTED".to_string(), encrypt_string("secret").unwrap());
        mixed.insert("PLAINTEXT".to_string(), "not-a-secret".to_string());

        let decrypted = decrypt_env_vars(&mixed);

        assert_eq!(decrypted.get("ENCRYPTED").unwrap(), "secret");
        assert_eq!(decrypted.get("PLAINTEXT").unwrap(), "not-a-secret");
    }

    #[test]
    fn test_invalid_format_errors() {
        setup_test_key();

        // Missing separator
        let bad1 = "<encrypted v=\"1\">no-separator</encrypted>";
        assert!(decrypt_string(bad1).is_err());

        // Invalid base64
        let bad2 = "<encrypted v=\"1\">!!!:!!!</encrypted>";
        assert!(decrypt_string(bad2).is_err());

        // Wrong nonce length (after base64 decode)
        let bad3 = "<encrypted v=\"1\">YWJj:ZGVm</encrypted>"; // "abc:def" in base64
        assert!(decrypt_string(bad3).is_err());
    }
}
