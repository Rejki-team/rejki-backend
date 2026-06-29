use serde::Deserialize;

/// Database connection configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL (required in production).
    pub url: String,
    /// Connection pool size.
    pub pool_size: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            pool_size: 10,
        }
    }
}
