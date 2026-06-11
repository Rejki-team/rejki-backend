//! Rate limiting permintaan OTP via Redis (K15 / spec otp-verification).
//!
//! Pola fixed-window: key `otp_req:{purpose}:{user_id}` di-INCR; pada increment
//! pertama dipasang EXPIRE = window. Bila count melampaui batas → ditolak.
//! Bila Redis tidak tersedia, fail-open (tidak memblokir) agar layanan tetap jalan,
//! dengan log peringatan — keputusan ini menjaga ketersediaan; batas percobaan
//! verifikasi (kolom attempts) tetap menjadi pertahanan utama anti brute-force.

use redis::AsyncCommands;

const WINDOW_SECS: i64 = 15 * 60; // 15 menit
const MAX_REQUESTS: i64 = 3; // maks 3 permintaan OTP per window per (user, purpose)

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
                        tracing::warn!(error = ?e, "REDIS_URL invalid — OTP rate limit non-aktif");
                        None
                    }
                });
        Self { client }
    }

    /// Kembalikan true bila permintaan diizinkan; false bila melampaui batas.
    pub async fn allow(&self, purpose: &str, user_key: &str) -> bool {
        let Some(client) = &self.client else {
            return true; // fail-open: Redis tidak dikonfigurasi
        };

        let key = format!("otp_req:{purpose}:{user_key}");
        match self.incr_and_check(client, &key).await {
            Ok(allowed) => allowed,
            Err(e) => {
                tracing::warn!(error = ?e, "rate limit check gagal — fail-open");
                true
            }
        }
    }

    async fn incr_and_check(
        &self,
        client: &redis::Client,
        key: &str,
    ) -> Result<bool, anyhow::Error> {
        let mut conn = client.get_multiplexed_async_connection().await?;
        let count: i64 = conn.incr(key, 1).await?;
        if count == 1 {
            let _: bool = conn.expire(key, WINDOW_SECS).await?;
        }
        Ok(count <= MAX_REQUESTS)
    }
}
