use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use clap::{Parser, Subcommand};
use rand::RngCore;
use rpassword::prompt_password;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

#[derive(Parser)]
#[command(name = "myrss-secrets")]
#[command(about = "Manage encrypted secrets for myrss", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, default_value = "secrets.yaml")]
    file: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        #[arg(help = "Key name for the secret")]
        key: String,
        #[arg(help = "Secret value (will prompt if not provided)")]
        value: Option<String>,
    },
    Get {
        #[arg(help = "Key name to retrieve")]
        key: String,
    },
    List,
    Remove {
        #[arg(help = "Key name to remove")]
        key: String,
    },
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
    let cli = Cli::parse();
    let mut store = SecretsStore::load(&cli.file)?;

    match cli.command {
        Commands::Add { key, value } => {
            let password = prompt_password("Enter master password: ")?;
            
            let secret_value = match value {
                Some(v) => v,
                None => prompt_password("Enter secret value: ")?,
            };

            let encrypted = encrypt_value(&secret_value, &password)?;
            store.secrets.insert(key.clone(), encrypted);
            store.save(&cli.file)?;
            println!("Secret '{}' added successfully", key);
        }
        Commands::Get { key } => {
            let password = prompt_password("Enter master password: ")?;
            
            match store.secrets.get(&key) {
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
        Commands::List => {
            println!("Stored secrets:");
            for key in store.secrets.keys() {
                println!("  - {}", key);
            }
        }
        Commands::Remove { key } => {
            if store.secrets.remove(&key).is_some() {
                store.save(&cli.file)?;
                println!("Secret '{}' removed successfully", key);
            } else {
                eprintln!("Secret '{}' not found", key);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}