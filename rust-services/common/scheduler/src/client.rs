use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::sync::Mutex;

use crate::envelope::JobEnvelope;
use crate::error::SchedulerError;
use crate::DELAYED_ZSET_KEY;

/// Sisi producer — dipanggil dari mana pun sebuah tugas terjadwal perlu didaftarkan
/// (mis. `iklan-pelatihan-service` menjadwalkan "tutup pendaftaran H-30 menit").
/// Fail-open sesuai pola `OtpRateLimiter`: bila `REDIS_URL` tidak diset, `schedule()`
/// mengembalikan `Err(Unconfigured)` — pemanggil WAJIB menangani ini sebagai degradasi
/// anggun (log `warn!`, jangan gagalkan operasi utama — §4.5 backend), sama seperti
/// pola notifier opsional yang sudah ada di `rejki-app/src/main.rs`.
#[derive(Clone)]
pub struct SchedulerClient {
    inner: Option<std::sync::Arc<Inner>>,
}

struct Inner {
    client: redis::Client,
    conn: Mutex<Option<ConnectionManager>>,
}

impl SchedulerClient {
    pub fn new(redis_url: Option<String>) -> Self {
        let inner = redis_url.and_then(|url| match redis::Client::open(url) {
            Ok(client) => Some(std::sync::Arc::new(Inner {
                client,
                conn: Mutex::new(None),
            })),
            Err(e) => {
                tracing::warn!(error = ?e, "REDIS_URL invalid — scheduler client non-aktif");
                None
            }
        });
        Self { inner }
    }

    async fn conn(&self) -> Result<ConnectionManager, SchedulerError> {
        let inner = self.inner.as_ref().ok_or(SchedulerError::Unconfigured)?;
        let mut guard = inner.conn.lock().await;
        if guard.is_none() {
            *guard = Some(inner.client.get_connection_manager().await?);
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    /// Jadwalkan `envelope` untuk dieksekusi setelah `envelope.execute_after`.
    /// Masuk ke sorted-set delay buffer (score = unix timestamp detik) — dipindahkan
    /// ke stream oleh `SchedulerConsumer::run_mover_tick` saat jatuh tempo.
    pub async fn schedule(&self, envelope: &JobEnvelope) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let member = serde_json::to_string(envelope)?;
        let score = envelope.execute_after.timestamp();
        let _: i64 = conn.zadd(DELAYED_ZSET_KEY, member, score).await?;
        tracing::debug!(
            job_id = %envelope.job_id,
            job_type = %envelope.job_type,
            execute_after = %envelope.execute_after,
            "job scheduled"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schedule_given_no_redis_url_when_called_then_returns_unconfigured() {
        let client = SchedulerClient::new(None);
        let env = JobEnvelope::new("noop", serde_json::json!({}), chrono::Utc::now());
        let result = client.schedule(&env).await;
        assert!(matches!(result, Err(SchedulerError::Unconfigured)));
    }
}
