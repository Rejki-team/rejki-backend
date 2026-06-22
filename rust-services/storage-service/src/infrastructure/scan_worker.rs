//! # Scan Worker — Virus scanner untuk file upload via ClamAV
//!
//! Worker background yang consume dari Redis Stream (`scan-queue`) untuk
//! mendeteksi file yang baru diupload ke MinIO, lalu memindainya via ClamAV.
//!
//! ## Flow
//! 1. MinIO bucket notification → XADD ke Redis Stream `scan-queue`
//! 2. Worker XREADGROUP dari `scan-queue` (consumer group: `scan-workers`)
//! 3. Download file dari MinIO (HTTP GET)
//! 4. Stream bytes ke ClamAV TCP (INSTREAM protocol)
//! 5. CLEAN: log info
//! 6. INFECTED: push notifikasi ke Redis `notifications:push`
//!
//! ## Graceful degradation
//! Jika ClamAV atau Redis tidak tersedia, worker akan log warning dan skip scan.
//! File upload tetap berjalan (fail-open) — scan adalah enhancement, bukan blocker.

use common_clamav::{ClamavClient, ScanResult};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tokio::sync::watch;

/// Prefix Redis Stream untuk queue scan
const SCAN_QUEUE: &str = "scan-queue";

/// Consumer group untuk scan workers
const SCAN_GROUP: &str = "scan-workers";

/// Prefix Redis Stream untuk push notification
const NOTIFICATION_STREAM: &str = "notifications:push";

/// Struct yang menyimpan koneksi Redis + ClamAV client
pub struct ScanWorker {
    redis: ConnectionManager,
    clamav: ClamavClient,
    minio: MinioClient,
    shutdown: watch::Receiver<bool>,
}

/// Client untuk download dari MinIO via HTTP GET
struct MinioClient {
    base_url: String,
    access_key: String,
    secret_key: String,
}

impl MinioClient {
    fn from_env() -> Option<Self> {
        let endpoint = std::env::var("MINIO_ENDPOINT").ok()?;
        let access_key = std::env::var("MINIO_ACCESS_KEY").ok()?;
        let secret_key = std::env::var("MINIO_SECRET_KEY").ok()?;
        let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "rejki-dokumen".into());
        let base_url = format!("{}/{}", endpoint.trim_end_matches('/'), bucket);
        Some(Self {
            base_url,
            access_key,
            secret_key,
        })
    }

    async fn download_object(&self, object_key: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}/{}", self.base_url, object_key);
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| format!("reqwest: {e}"))?;

        let resp = client
            .get(&url)
            .header(
                "Authorization",
                format!("AWS {}:{}", self.access_key, self.secret_key),
            )
            .send()
            .await
            .map_err(|e| format!("download: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("read: {e}"))
    }
}

impl ScanWorker {
    /// Buat ScanWorker dari env vars.
    /// Returns `None` jika Redis atau ClamAV tidak dikonfigurasi (fail-open).
    pub async fn from_env(shutdown: watch::Receiver<bool>) -> Option<Self> {
        let redis_url = std::env::var("REDIS_URL").ok()?;
        let redis_client = redis::Client::open(redis_url.as_str()).ok()?;
        let redis = redis_client.get_connection_manager().await.ok()?;
        let clamav = ClamavClient::from_env();
        let minio = MinioClient::from_env()?;

        Some(Self {
            redis,
            clamav,
            minio,
            shutdown,
        })
    }

    /// Jalankan worker loop — blocking sampai shutdown signal.
    pub async fn run(&self) {
        tracing::info!("scan worker: starting (ClamAV consumer)");

        // Buat consumer group (MKSTREAM = create stream jika belum ada)
        let _ = self.create_consumer_group().await;

        let consumer_id = format!("scan-worker-{}", std::process::id());

        loop {
            if *self.shutdown.borrow() {
                tracing::info!("scan worker: shutdown signal received");
                break;
            }

            // Baca 1 event dari Redis Stream (block max 5 detik)
            match self.read_next_event(&consumer_id).await {
                Ok(Some((entry_id, fields))) => {
                    self.process_event(&entry_id, &fields).await;
                    // XACK setelah selesai
                    let _: Result<(), _> = self
                        .redis
                        .clone()
                        .xack(SCAN_QUEUE, SCAN_GROUP, &[entry_id.as_str()])
                        .await;
                }
                Ok(None) => {
                    // Timeout, loop lagi
                    continue;
                }
                Err(e) => {
                    tracing::warn!("scan worker: redis error: {e}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// Buat consumer group (jika sudah ada, error di-ignore).
    async fn create_consumer_group(&self) {
        let mut conn = self.redis.clone();
        let mut cmd = redis::cmd("XGROUP");
        cmd.arg("CREATE")
            .arg(SCAN_QUEUE)
            .arg(SCAN_GROUP)
            .arg("$")
            .arg("MKSTREAM");

        match cmd.query_async::<()>(&mut conn).await {
            Ok(_) => tracing::info!("scan worker: consumer group created"),
            Err(e) => tracing::warn!("scan worker: consumer group exists: {e}"),
        }
    }

    /// Baca event berikutnya dari Redis Stream.
    /// Returns Ok(Some((id, fields))) jika ada event, Ok(None) jika timeout.
    async fn read_next_event(
        &self,
        consumer_id: &str,
    ) -> Result<Option<(String, Vec<(String, String)>)>, redis::RedisError> {
        let mut conn = self.redis.clone();
        #[allow(clippy::type_complexity)]
        let results: Vec<(String, Vec<(String, Vec<(String, String)>)>)> = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(SCAN_GROUP)
            .arg(consumer_id)
            .arg("COUNT")
            .arg("1")
            .arg("BLOCK")
            .arg("5000")
            .arg("STREAMS")
            .arg(SCAN_QUEUE)
            .arg(">")
            .query_async(&mut conn)
            .await?;

        Ok(results
            .into_iter()
            .next()
            .and_then(|(_stream, entries)| entries.into_iter().next()))
    }

    /// Process satu event upload.
    async fn process_event(&self, entry_id: &str, fields: &[(String, String)]) {
        let fields_map: std::collections::HashMap<&str, &str> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let object_key = match fields_map.get("key") {
            Some(k) => *k,
            None => {
                tracing::warn!("scan worker: event tanpa 'key' field: {entry_id}");
                return;
            }
        };

        tracing::info!("scan worker: processing {object_key}");

        let file_bytes = match self.minio.download_object(object_key).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("scan worker: download failed {object_key}: {e}");
                return;
            }
        };

        if file_bytes.is_empty() {
            tracing::warn!("scan worker: empty file {object_key}");
            return;
        }

        match self.clamav.scan_bytes(&file_bytes).await {
            Ok(ScanResult::Clean) => {
                tracing::info!("scan worker: {object_key} → CLEAN");
            }
            Ok(ScanResult::Infected(virus)) => {
                tracing::warn!("scan worker: {object_key} → INFECTED ({virus})");
                self.notify_infected(object_key, &virus).await;
            }
            Err(e) => {
                tracing::error!("scan worker: scan failed {object_key}: {e}");
            }
        }
    }

    async fn notify_infected(&self, object_key: &str, virus: &str) {
        let payload = format!(
            r#"{{"event":"file.scan.infected","object_key":"{}","virus":"{}","timestamp":"{}"}}"#,
            object_key,
            virus,
            chrono::Utc::now().to_rfc3339()
        );

        let _: Result<(), _> = self
            .redis
            .clone()
            .xadd(NOTIFICATION_STREAM, "*", &[("event", payload.as_str())])
            .await;

        tracing::info!("scan worker: notification pushed for {object_key}");
    }
}
