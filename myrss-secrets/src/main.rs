use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use clap::{Arg, Command};
use rand::RngCore;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::io::{self, Read};

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedValue {
    ciphertext: String,
    nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecretsStore {
    secrets: HashMap<String, EncryptedValue>,
}

impl SecretsStore {
    fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    fn load(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path)
                .context("Failed to read secrets file")?;
            serde_yaml::from_str(&content)
                .context("Failed to parse secrets file")
        } else {
            Ok(Self::new())
        }
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_yaml::to_string(self)
            .context("Failed to serialize secrets")?;
        fs::write(path, content)
            .context("Failed to write secrets file")?;
        Ok(())
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

fn encrypt_value(value: &str, password: &str) -> Result<EncryptedValue> {
    let key = derive_key_from_password(password);
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .context("Failed to create cipher")?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, value.as_bytes())
        .map_err(|_| anyhow::anyhow!("Failed to encrypt value"))?;

    Ok(EncryptedValue {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(&nonce_bytes),
    })
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

fn main() -> Result<()> {
    let app = Command::new("myrss-secrets")
        .about("Manage encrypted secrets for myrss")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .default_value("secrets.yaml")
                .help("Path to the secrets file")
        )
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Add a new secret")
                .arg(
                    Arg::new("key")
                        .help("Key name for the secret")
                        .required(true)
                        .index(1)
                )
                .arg(
                    Arg::new("value")
                        .help("Secret value (will prompt if not provided)")
                        .index(2)
                )
        )
        .subcommand(
            Command::new("get")
                .about("Get a secret value")
                .arg(
                    Arg::new("key")
                        .help("Key name to retrieve")
                        .required(true)
                        .index(1)
                )
        )
        .subcommand(
            Command::new("list")
                .about("List all secret keys")
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a secret")
                .arg(
                    Arg::new("key")
                        .help("Key name to remove")
                        .required(true)
                        .index(1)
                )
        );

    let matches = app.get_matches();
    let file = PathBuf::from(matches.get_one::<String>("file").unwrap());
    let mut store = SecretsStore::load(&file)?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap().clone();
            let value = sub_matches.get_one::<String>("value").map(|s| s.clone());
            
            // Check for master password in environment variable first
            let env_password = std::env::var("MYRSS_MASTER_PASSWORD").ok();
            let password = match &env_password {
                Some(p) => p.clone(),
                None => prompt_password("Enter master password: ")?,
            };
            
            let secret_value = match value {
                Some(v) => v,
                None => {
                    // If password is from env, assume we're in automation mode and read from stdin
                    if env_password.is_some() {
                        let mut buffer = String::new();
                        io::stdin().read_to_string(&mut buffer)?;
                        buffer.trim().to_string()
                    } else {
                        prompt_password("Enter secret value: ")?
                    }
                },
            };

            let encrypted = encrypt_value(&secret_value, &password)?;
            store.secrets.insert(key.clone(), encrypted);
            store.save(&file)?;
            println!("Secret '{}' added successfully", key);
        }
        Some(("get", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap();
            
            // Check for master password in environment variable first
            let password = match std::env::var("MYRSS_MASTER_PASSWORD") {
                Ok(p) => p,
                Err(_) => prompt_password("Enter master password: ")?,
            };
            
            match store.secrets.get(key) {
                Some(encrypted) => {
                    let value = decrypt_value(encrypted, &password)?;
                    println!("{}", value);
                }
                None => {
                    eprintln!("Secret '{}' not found", key);
                    std::process::exit(1);
                }
            }
        }
        Some(("list", _)) => {
            println!("Stored secrets:");
            for key in store.secrets.keys() {
                println!("  - {}", key);
            }
        }
        Some(("remove", sub_matches)) => {
            let key = sub_matches.get_one::<String>("key").unwrap();
            if store.secrets.remove(key).is_some() {
                store.save(&file)?;
                println!("Secret '{}' removed successfully", key);
            } else {
                eprintln!("Secret '{}' not found", key);
                std::process::exit(1);
            }
        }
        _ => unreachable!("Subcommand required"),
    }

    Ok(())
}