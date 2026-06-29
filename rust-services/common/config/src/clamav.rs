use serde::Deserialize;

/// ClamAV antivirus configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ClamavConfig {
    /// ClamAV daemon host (default: `clamav-daemon` — Docker service name).
    pub host: String,
    /// ClamAV daemon port (default: 3310).
    pub port: u16,
}

impl Default for ClamavConfig {
    fn default() -> Self {
        Self {
            host: "clamav-daemon".into(),
            port: 3310,
        }
    }
}
