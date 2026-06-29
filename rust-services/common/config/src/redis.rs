use serde::Deserialize;

/// Redis connection configuration.
/// Optional: jika tidak diset, rate limiter dan scan worker akan non-aktif (fail-open).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct RedisConfig {
    /// Redis URL, e.g., `redis://redis:6379/0`.
    pub url: Option<String>,
}
