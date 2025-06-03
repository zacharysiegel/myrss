use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedValue {
    ciphertext: String,
    nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecretsStore {
    secrets: HashMap<String, EncryptedValue>,
}

pub struct SecretsReader {
    store: SecretsStore,
    password: String,
}

impl SecretsReader {
    pub fn new<P: AsRef<Path>>(path: P, password: String) -> Result<Self> {
        let content = fs::read_to_string(path)
            .context("Failed to read secrets file")?;
        let store: SecretsStore = serde_yaml::from_str(&content)
            .context("Failed to parse secrets file")?;
        
        Ok(Self { store, password })
    }

    pub fn get(&self, key: &str) -> Result<String> {
        let encrypted = self.store.secrets
            .get(key)
            .context(format!("Secret '{}' not found", key))?;
        
        decrypt_value(encrypted, &self.password)
    }

    pub fn get_or_default(&self, key: &str, default: String) -> String {
        self.get(key).unwrap_or(default)
    }
}

fn derive_key_from_password(password: &str) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(b"myrss-secret-key-derivation");
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn decrypt_value(encrypted: &EncryptedValue, password: &str) -> Result<String> {
    let key = derive_key_from_password(password);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .context("Failed to create cipher")?;

    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .context("Failed to decode ciphertext")?;
    let nonce_bytes = BASE64
        .decode(&encrypted.nonce)
        .context("Failed to decode nonce")?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Failed to decrypt value"))?;

    String::from_utf8(plaintext)
        .context("Failed to convert decrypted value to string")
}