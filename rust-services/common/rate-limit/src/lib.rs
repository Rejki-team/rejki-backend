//! Shared rate limiter — Redis Lua script atomic INCR + conditional EXPIRE.
//!
//! Dipakai oleh semua service yang butuh rate limiting (auth, iklan, report, chat, notification).
//! Pola: fixed-window atomik via Lua script. Fail-open: bila Redis tidak tersedia, semua request diizinkan.
//!
//! Key format: `rl:{service}:{purpose}:{user_id}` (atau custom key via `allow_raw`)

/// Trait rate limiter — bisa di-inject via `Arc<dyn RateLimiter>`.
#[async_trait::async_trait]
pub trait RateLimiter: Send + Sync {
    /// Cek apakah request dengan `user_key` dan `purpose` diizinkan.
    /// Return true = allowed, false = rate limited.
    async fn allow(&self, purpose: &str, user_key: &str) -> bool;

    /// Versi raw — menerima key custom dan parameter rate limit.
    /// Return true = allowed, false = rate limited.
    async fn allow_raw(&self, key: &str, max_requests: i64, window_secs: i64) -> bool;
}

/// Lua script: INCR + conditional EXPIRE (hanya pada count==1) dalam satu operasi atomik.
/// KEYS[1] = rate limit key
/// ARGV[1] = max_requests
/// ARGV[2] = window_secs
/// Return: 1 = allowed, 0 = denied
const RATE_LIMIT_LUA: &str = r#"
local count = redis.call('INCR', KEYS[1])
if count == 1 then
    redis.call('EXPIRE', KEYS[1], ARGV[2])
end
if count <= tonumber(ARGV[1]) then
    return 1
else
    return 0
end
"#;

/// Default window untuk rate limit bisnis (15 menit).
pub const BISNIS_WINDOW_SECS: i64 = 15 * 60;
/// Default max request untuk endpoint create (30 req per window).
pub const BISNIS_CREATE_MAX: i64 = 30;
/// Tighter limit untuk create di service sensitif (10 req per window).
pub const BISNIS_CREATE_TIGHT: i64 = 10;
/// Rate limit key prefix untuk endpoint bisnis.
pub const BISNIS_KEY_PREFIX: &str = "rl";

#[derive(Clone)]
pub struct OtpRateLimiter {
    client: Option<redis::Client>,
}

impl OtpRateLimiter {
    /// Bangun dari REDIS_URL; bila tidak diset/invalid → limiter non-aktif (fail-open).
    pub fn from_env() -> Self {
        let client =
            std::env::var("REDIS_URL")
                .ok()
                .and_then(|url| match redis::Client::open(url) {
                    Ok(c) => Some(c),
                    Err(e) => {
                        tracing::warn!(error = ?e, "REDIS_URL invalid — rate limiter non-aktif");
                        None
                    }
                });
        Self { client }
    }

    /// Kembalikan true bila permintaan diizinkan; false bila melampaui batas.
    /// Format key: `rl:{service}:{purpose}:{user_key}`
    pub async fn allow(&self, purpose: &str, user_key: &str) -> bool {
        let key = format!("{BISNIS_KEY_PREFIX}:{purpose}:{user_key}");
        self.allow_raw(&key, BISNIS_CREATE_MAX, BISNIS_WINDOW_SECS)
            .await
    }

    /// Versi raw — menerima key dan parameter rate limit kustom.
    pub async fn allow_raw(&self, key: &str, max_requests: i64, window_secs: i64) -> bool {
        let Some(client) = &self.client else {
            return true; // fail-open: Redis tidak dikonfigurasi
        };

        match self.eval_lua(client, key, max_requests, window_secs).await {
            Ok(allowed) => allowed,
            Err(e) => {
                tracing::warn!(error = ?e, "rate limit check gagal — fail-open");
                true
            }
        }
    }

    /// Evaluasi Lua script atomik via EVALSHA (dengan fallback ke EVAL).
    async fn eval_lua(
        &self,
        client: &redis::Client,
        key: &str,
        max_requests: i64,
        window_secs: i64,
    ) -> Result<bool, anyhow::Error> {
        let mut conn = client.get_multiplexed_async_connection().await?;
        let script = redis::Script::new(RATE_LIMIT_LUA);
        let result: i32 = script
            .key(key)
            .arg(max_requests)
            .arg(window_secs)
            .invoke_async(&mut conn)
            .await?;
        Ok(result == 1)
    }
}

#[async_trait::async_trait]
impl RateLimiter for OtpRateLimiter {
    async fn allow(&self, purpose: &str, user_key: &str) -> bool {
        OtpRateLimiter::allow(self, purpose, user_key).await
    }

    async fn allow_raw(&self, key: &str, max_requests: i64, window_secs: i64) -> bool {
        OtpRateLimiter::allow_raw(self, key, max_requests, window_secs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisnis_key_prefix_is_exported() {
        assert_eq!(BISNIS_KEY_PREFIX, "rl");
    }

    #[test]
    fn from_env_no_redis_url_returns_no_client() {
        std::env::remove_var("REDIS_URL");
        let limiter = OtpRateLimiter::from_env();
        assert!(limiter.client.is_none());
    }

    #[tokio::test]
    async fn allow_returns_true_when_no_client() {
        let limiter = OtpRateLimiter { client: None };
        assert!(
            limiter
                .allow("iklan_pekerjaan:create", "test-user-id")
                .await
        );
    }

    #[tokio::test]
    async fn allow_raw_returns_true_when_no_client() {
        let limiter = OtpRateLimiter { client: None };
        assert!(limiter.allow_raw("test_key", 5, 900).await);
    }

    #[test]
    fn lua_script_is_valid() {
        assert!(!RATE_LIMIT_LUA.is_empty());
        assert!(RATE_LIMIT_LUA.contains("INCR"));
        assert!(RATE_LIMIT_LUA.contains("EXPIRE"));
    }

    #[test]
    fn constants_are_reasonable() {
        assert!(BISNIS_CREATE_MAX >= 10 && BISNIS_CREATE_MAX <= 100);
        assert!(BISNIS_CREATE_TIGHT >= 1 && BISNIS_CREATE_TIGHT <= 30);
        assert!(BISNIS_WINDOW_SECS >= 60);
    }
}
