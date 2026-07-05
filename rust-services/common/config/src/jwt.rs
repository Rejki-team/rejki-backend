use serde::Deserialize;
use std::path::PathBuf;

/// JWT (RS256) configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct JwtConfig {
    /// Path ke private key PEM file.
    pub private_key_path: PathBuf,
    /// Path ke public key PEM file.
    pub public_key_path: PathBuf,
    /// Access token TTL in seconds (default: 900 = 15 menit).
    pub access_ttl_secs: i64,
    /// Refresh token TTL in seconds (default: 2_592_000 = 30 hari).
    pub refresh_ttl_secs: i64,
}

impl JwtConfig {
    /// Load both key files as strings.
    pub fn load_keys(&self) -> anyhow::Result<(String, String)> {
        let private_pem = std::fs::read_to_string(&self.private_key_path).map_err(|e| {
            anyhow::anyhow!(
                "JWT_PRIVATE_KEY_PATH ({:?}) tidak terbaca: {e}",
                self.private_key_path
            )
        })?;
        let public_pem = std::fs::read_to_string(&self.public_key_path).map_err(|e| {
            anyhow::anyhow!(
                "JWT_PUBLIC_KEY_PATH ({:?}) tidak terbaca: {e}",
                self.public_key_path
            )
        })?;
        Ok((private_pem, public_pem))
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            private_key_path: PathBuf::from("./keys/private.pem"),
            public_key_path: PathBuf::from("./keys/public.pem"),
            access_ttl_secs: 900,
            refresh_ttl_secs: 2_592_000,
        }
    }
}
