use anyhow::Result;
use myrss_secrets::SecretsReader;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub session_key: String,
    pub auth_header: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let secrets_file = std::env::var("MYRSS_SECRETS_FILE")
            .unwrap_or_else(|_| "secrets.yaml".to_string());
        let master_password = std::env::var("MYRSS_MASTER_PASSWORD")
            .expect("MYRSS_MASTER_PASSWORD must be set");

        let secrets = SecretsReader::new(&secrets_file, master_password)?;

        Ok(Config {
            database_url: secrets.get_or_default(
                "database_url",
                "postgresql://myrss:myrss@localhost/myrss".to_string()
            ),
            host: std::env::var("MYRSS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("MYRSS_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            session_key: secrets.get("session_key")?,
            auth_header: std::env::var("MYRSS_AUTH_HEADER")
                .unwrap_or_else(|_| "X-Authenticated-User".to_string()),
        })
    }
}