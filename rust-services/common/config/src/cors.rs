use serde::Deserialize;

/// CORS configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    /// Comma-separated list of allowed origins.
    /// Default (development): `http://localhost:5173`
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:5173".to_string()],
        }
    }
}
