use std::sync::Arc;
use std::time::Duration;

use redis::aio::ConnectionManager;
use redis::streams::{StreamAutoClaimOptions, StreamReadOptions};
use redis::{AsyncCommands, ExistenceCheck, SetExpiry, SetOptions};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::envelope::JobEnvelope;
use crate::error::SchedulerError;
use crate::registry::SchedulerRegistry;
use crate::{
    BATCH_SIZE, BLOCK_MS, CONSUMER_GROUP, DELAYED_ZSET_KEY, DLQ_STREAM_KEY, IDEMPOTENCY_PREFIX,
    MAX_RETRY, MOVER_BATCH, MOVER_INTERVAL, PROCESSED_TTL_SECS, RECLAIM_MIN_IDLE_MS, STREAM_KEY,
};

/// Lua script: pindahkan job yang jatuh tempo (`score <= now`) dari sorted-set delay
/// buffer ke stream — SATU `EVAL` atomik (Redis mengeksekusi Lua single-threaded),
/// aman dijalankan bersamaan oleh banyak instance `SchedulerConsumer` tanpa lock
/// tambahan (Hazard #1) — `ZREM` yang gagal (member sudah diambil instance lain)
/// otomatis dilewati.
const MOVER_LUA: &str = r#"
local due = redis.call('ZRANGEBYSCORE', KEYS[1], '-inf', ARGV[1], 'LIMIT', 0, tonumber(ARGV[2]))
local moved = 0
for _, member in ipairs(due) do
    local removed = redis.call('ZREM', KEYS[1], member)
    if removed == 1 then
        redis.call('XADD', KEYS[2], '*', 'payload', member)
        moved = moved + 1
    end
end
return moved
"#;

/// Hitung durasi backoff eksponensial + jitter untuk percobaan ke-`attempt`
/// (1-based). Base 500ms, dikali 2^(attempt-1), ditambah jitter 0-250ms —
/// §4.5 backend: "retry aman: idempoten, exponential backoff + jitter".
pub(crate) fn backoff_duration(attempt: u32) -> Duration {
    let base_ms = 500u64.saturating_mul(1u64 << (attempt.saturating_sub(1)).min(10));
    let jitter_ms = rand::random::<u64>() % 250;
    Duration::from_millis(base_ms.min(30_000) + jitter_ms)
}

/// Sisi consumer generik — `XREADGROUP` + mover tick + retry/backoff + DLQ +
/// idempotency. Satu instance per proses (dijalankan sebagai background task di
/// `rejki-app`); aman multi-instance (Hazard #1) lewat semantik consumer-group
/// Redis Streams (setiap entry stream hanya dikirim ke SATU consumer dalam grup).
pub struct SchedulerConsumer {
    inner: Arc<Inner>,
    registry: SchedulerRegistry,
    consumer_name: String,
}

struct Inner {
    client: redis::Client,
    conn: Mutex<Option<ConnectionManager>>,
}

impl SchedulerConsumer {
    pub fn new(
        redis_url: &str,
        registry: SchedulerRegistry,
        consumer_name: impl Into<String>,
    ) -> Result<Self, SchedulerError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            inner: Arc::new(Inner {
                client,
                conn: Mutex::new(None),
            }),
            registry,
            consumer_name: consumer_name.into(),
        })
    }

    async fn conn(&self) -> Result<ConnectionManager, SchedulerError> {
        let mut guard = self.inner.conn.lock().await;
        if guard.is_none() {
            *guard = Some(self.inner.client.get_connection_manager().await?);
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    /// Buat consumer-group bila belum ada (idempoten — `BUSYGROUP` diabaikan). Publik
    /// agar test integrasi bisa menyiapkan stream tanpa menjalankan `run()` (yang tidak
    /// pernah `return`).
    pub async fn ensure_consumer_group(&self) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let result: redis::RedisResult<()> = conn
            .xgroup_create_mkstream(STREAM_KEY, CONSUMER_GROUP, "0")
            .await;
        if let Err(e) = result {
            if !e.to_string().contains("BUSYGROUP") {
                return Err(e.into());
            }
        }
        Ok(())
    }

    /// Pindahkan job yang jatuh tempo dari delay buffer ke stream. Dipanggil berkala
    /// dari `run()`; dapat juga dipanggil manual (mis. dari test) untuk memicu
    /// pemindahan tanpa menunggu interval.
    pub async fn run_mover_tick(&self) -> Result<u32, SchedulerError> {
        let mut conn = self.conn().await?;
        let now = chrono::Utc::now().timestamp();
        let script = redis::Script::new(MOVER_LUA);
        let moved: i64 = script
            .key(DELAYED_ZSET_KEY)
            .key(STREAM_KEY)
            .arg(now)
            .arg(MOVER_BATCH)
            .invoke_async(&mut conn)
            .await?;
        if moved > 0 {
            tracing::debug!(moved, "scheduler: moved due jobs to stream");
        }
        Ok(moved as u32)
    }

    async fn is_processed(&self, job_id: Uuid) -> Result<bool, SchedulerError> {
        let mut conn = self.conn().await?;
        let key = format!("{IDEMPOTENCY_PREFIX}{job_id}");
        Ok(conn.exists(key).await?)
    }

    async fn mark_processed(&self, job_id: Uuid) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let key = format!("{IDEMPOTENCY_PREFIX}{job_id}");
        let opts = SetOptions::default()
            .conditional_set(ExistenceCheck::NX)
            .with_expiration(SetExpiry::EX(PROCESSED_TTL_SECS));
        let _: Option<String> = conn.set_options(key, "1", opts).await?;
        Ok(())
    }

    async fn move_to_dlq(
        &self,
        original_id: &str,
        payload: &str,
        error: &str,
    ) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let _: String = conn
            .xadd(
                DLQ_STREAM_KEY,
                "*",
                &[
                    ("original_id", original_id),
                    ("payload", payload),
                    ("error", error),
                    ("failed_at", &chrono::Utc::now().to_rfc3339()),
                ],
            )
            .await?;
        Ok(())
    }

    async fn ack(&self, stream_id: &str) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let _: i64 = conn.xack(STREAM_KEY, CONSUMER_GROUP, &[stream_id]).await?;
        Ok(())
    }

    /// Proses satu entry stream sampai tuntas: idempotency check → dispatch handler
    /// (retry+backoff maks `MAX_RETRY`) → ack, atau DLQ bila payload invalid/handler
    /// tidak ditemukan/gagal terus.
    async fn handle_message(&self, stream_id: &str, payload: &str) {
        let envelope: JobEnvelope = match serde_json::from_str(payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(stream_id, error = ?e, "scheduler: payload tidak valid — DLQ langsung");
                let _ = self
                    .move_to_dlq(stream_id, payload, &format!("invalid payload: {e}"))
                    .await;
                let _ = self.ack(stream_id).await;
                return;
            }
        };

        match self.is_processed(envelope.job_id).await {
            Ok(true) => {
                tracing::debug!(job_id = %envelope.job_id, "scheduler: job sudah diproses (idempotent) — skip");
                let _ = self.ack(stream_id).await;
                return;
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(error = ?e, "scheduler: gagal cek idempotency marker — lanjut proses (fail-open)");
            }
        }

        let handler = match self.registry.get(&envelope.job_type) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!(job_type = %envelope.job_type, "scheduler: {e} — DLQ langsung");
                let _ = self.move_to_dlq(stream_id, payload, &e.to_string()).await;
                let _ = self.ack(stream_id).await;
                return;
            }
        };

        let mut attempt = 0u32;
        loop {
            match handler.handle(&envelope.payload).await {
                Ok(()) => {
                    if let Err(e) = self.mark_processed(envelope.job_id).await {
                        tracing::warn!(error = ?e, "scheduler: gagal set idempotency marker (non-fatal)");
                    }
                    let _ = self.ack(stream_id).await;
                    return;
                }
                Err(e) => {
                    attempt += 1;
                    if attempt > MAX_RETRY {
                        tracing::error!(
                            job_id = %envelope.job_id,
                            job_type = %envelope.job_type,
                            error = ?e,
                            "scheduler: gagal setelah {MAX_RETRY} retry — DLQ"
                        );
                        let _ = self.move_to_dlq(stream_id, payload, &e.to_string()).await;
                        let _ = self.ack(stream_id).await;
                        return;
                    }
                    let backoff = backoff_duration(attempt);
                    tracing::warn!(
                        job_id = %envelope.job_id,
                        attempt,
                        backoff_ms = backoff.as_millis(),
                        error = ?e,
                        "scheduler: retry job"
                    );
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// `XAUTOCLAIM` job yang stuck di PEL consumer lain (mis. crash) lebih dari
    /// `RECLAIM_MIN_IDLE_MS`. Dipanggil sekali saat startup (mirror pola
    /// `reclaim_stale()` bun-notification-service).
    pub async fn reclaim_stale(&self) -> Result<(), SchedulerError> {
        let mut conn = self.conn().await?;
        let opts = StreamAutoClaimOptions::default().count(MOVER_BATCH as usize);
        let reply: redis::streams::StreamAutoClaimReply = conn
            .xautoclaim_options(
                STREAM_KEY,
                CONSUMER_GROUP,
                &self.consumer_name,
                RECLAIM_MIN_IDLE_MS,
                "0-0",
                opts,
            )
            .await?;
        drop(conn);
        for entry in reply.claimed {
            if let Some(payload) = extract_payload(&entry.map) {
                self.handle_message(&entry.id, &payload).await;
            }
        }
        Ok(())
    }

    async fn read_batch(&self) -> Result<Vec<(String, String)>, SchedulerError> {
        let mut conn = self.conn().await?;
        let opts = StreamReadOptions::default()
            .group(CONSUMER_GROUP, &self.consumer_name)
            .count(BATCH_SIZE as usize)
            .block(BLOCK_MS as usize);
        let reply: redis::streams::StreamReadReply =
            conn.xread_options(&[STREAM_KEY], &[">"], &opts).await?;
        let mut out = Vec::new();
        for stream_key in reply.keys {
            for entry in stream_key.ids {
                if let Some(payload) = extract_payload(&entry.map) {
                    out.push((entry.id, payload));
                }
            }
        }
        Ok(out)
    }

    /// Baca satu batch (`XREADGROUP`, `BLOCK_MS`) dan proses semuanya sampai tuntas
    /// (ack/DLQ). Return jumlah entry yang diproses. Publik agar test integrasi bisa
    /// memicu satu iterasi tanpa menjalankan `run()` (yang tidak pernah `return`).
    pub async fn process_one_batch(&self) -> Result<usize, SchedulerError> {
        let messages = self.read_batch().await?;
        for (id, payload) in &messages {
            self.handle_message(id, payload).await;
        }
        Ok(messages.len())
    }

    /// Loop utama: mover tick berkala + `XREADGROUP` blocking. Dipanggil sekali dari
    /// composition root sebagai background task (`tokio::spawn`) — tidak pernah
    /// `return` normal, mirror `run_consumer(): Promise<never>` bun-notification-service.
    pub async fn run(&self) -> Result<(), SchedulerError> {
        self.ensure_consumer_group().await?;
        self.reclaim_stale().await?;
        tracing::info!(consumer = %self.consumer_name, "scheduler consumer started");

        let mut mover_tick = tokio::time::interval(MOVER_INTERVAL);
        loop {
            tokio::select! {
                _ = mover_tick.tick() => {
                    if let Err(e) = self.run_mover_tick().await {
                        tracing::warn!(error = ?e, "scheduler: mover tick gagal");
                    }
                }
                result = self.read_batch() => {
                    match result {
                        Ok(messages) => {
                            for (id, payload) in messages {
                                self.handle_message(&id, &payload).await;
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = ?e, "scheduler: XREADGROUP gagal — tunggu sebentar sebelum retry");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            }
        }
    }
}

fn extract_payload(map: &std::collections::HashMap<String, redis::Value>) -> Option<String> {
    map.get("payload")
        .and_then(|v| redis::from_redis_value::<String>(v).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_duration_given_increasing_attempts_when_computed_then_grows() {
        let d1 = backoff_duration(1).as_millis();
        let d2 = backoff_duration(2).as_millis();
        // Base tanpa jitter: attempt 1 -> 500ms, attempt 2 -> 1000ms. Dengan jitter
        // (0-250ms) perbandingan langsung tidak selalu > tapi rentang bawahnya harus naik.
        assert!((500..750).contains(&d1));
        assert!((1000..1250).contains(&d2));
    }

    #[test]
    fn test_backoff_duration_given_large_attempt_when_computed_then_capped() {
        let d = backoff_duration(20).as_millis();
        assert!(d <= 30_250);
    }

    #[test]
    fn test_mover_lua_script_is_valid() {
        assert!(MOVER_LUA.contains("ZRANGEBYSCORE"));
        assert!(MOVER_LUA.contains("ZREM"));
        assert!(MOVER_LUA.contains("XADD"));
    }
}
