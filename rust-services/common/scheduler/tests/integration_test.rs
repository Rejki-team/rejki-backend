//! Integration test terhadap Redis NYATA (bukan mock) — P5.4 Kelompok 2 Phase 5.
//! Butuh Redis hidup di `TEST_REDIS_URL` (default: container verify lokal).
//!
//! Digabung jadi SATU test function (bukan beberapa `#[tokio::test]` terpisah):
//! semuanya memakai key Redis global yang sama (`scheduler:delayed`/`scheduler:jobs`/
//! `scheduler:jobs:dlq`, konstan modul — bukan per-instance), dan `cargo test`
//! menjalankan test dalam thread paralel secara default — dua test terpisah akan
//! saling menimpa state satu sama lain. Sekuensial dalam satu fungsi menghindari
//! race ini tanpa perlu dependency tambahan (`serial_test`) atau flag CLI khusus.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use common_scheduler::{
    JobEnvelope, JobHandler, SchedulerClient, SchedulerConsumer, SchedulerRegistry, DLQ_STREAM_KEY,
    STREAM_KEY,
};
use redis::AsyncCommands;

const TEST_REDIS_URL: &str = "redis://127.0.0.1:6379";

async fn flush_test_keys() {
    let client = redis::Client::open(TEST_REDIS_URL).expect("redis url invalid");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("gagal konek redis verify — pastikan container redis-verify hidup");
    let _: () = redis::cmd("DEL")
        .arg(common_scheduler::DELAYED_ZSET_KEY)
        .arg(STREAM_KEY)
        .arg(DLQ_STREAM_KEY)
        .query_async(&mut conn)
        .await
        .unwrap();
}

struct CountingHandler {
    count: Arc<AtomicU32>,
}

#[async_trait::async_trait]
impl JobHandler for CountingHandler {
    async fn handle(&self, _payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct AlwaysFailHandler;

#[async_trait::async_trait]
impl JobHandler for AlwaysFailHandler {
    async fn handle(&self, _payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        Err(anyhow::anyhow!("simulated failure"))
    }
}

/// Lifecycle penuh scheduler generik (P5.4):
/// 1. Job jatuh tempo (execute_after masa lalu) → mover memindahkannya ke stream.
/// 2. `process_one_batch` memanggil handler tepat sekali, ack entry.
/// 3. Re-delivery entry yang sama (job_id identik) → idempotency marker mencegah
///    handler terpanggil lagi, entry tetap ter-ack (tidak macet).
/// 4. Job dengan handler yang selalu gagal → setelah `MAX_RETRY` percobaan, masuk
///    DLQ dan entry asli tetap ter-`XACK` (tidak macet selamanya di PEL).
#[tokio::test]
async fn test_scheduler_generic_component_lifecycle_idempotency_and_dlq() {
    flush_test_keys().await;

    // ── 1-3: job sukses + idempotency ───────────────────────────────────────
    let count = Arc::new(AtomicU32::new(0));
    let mut registry = SchedulerRegistry::new();
    registry.register(
        "count_test",
        Arc::new(CountingHandler {
            count: count.clone(),
        }) as Arc<dyn JobHandler>,
    );

    let client = SchedulerClient::new(Some(TEST_REDIS_URL.to_string()));
    let env = JobEnvelope::new(
        "count_test",
        serde_json::json!({"x": 1}),
        chrono::Utc::now() - chrono::Duration::seconds(5),
    );
    client.schedule(&env).await.expect("schedule harus sukses");

    let consumer = SchedulerConsumer::new(TEST_REDIS_URL, registry, "test-consumer")
        .expect("consumer harus terbentuk");
    consumer
        .ensure_consumer_group()
        .await
        .expect("ensure group");

    let moved = consumer.run_mover_tick().await.expect("mover tick");
    assert_eq!(moved, 1, "job yang jatuh tempo harus terpindah ke stream");

    let processed = consumer.process_one_batch().await.expect("process batch");
    assert_eq!(processed, 1);
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "handler harus terpanggil tepat sekali"
    );

    // Simulasikan re-delivery entry yang sama (mis. setelah reconnect/XAUTOCLAIM) —
    // idempotency marker (dicek SEBELUM handler, bukan hanya audit trail) harus
    // mencegah handler terpanggil lagi.
    {
        let redis_client = redis::Client::open(TEST_REDIS_URL).unwrap();
        let mut conn = redis_client
            .get_multiplexed_async_connection()
            .await
            .unwrap();
        let payload = serde_json::to_string(&env).unwrap();
        let _: String = conn
            .xadd(STREAM_KEY, "*", &[("payload", payload)])
            .await
            .unwrap();
    }
    let processed2 = consumer.process_one_batch().await.expect("process batch 2");
    assert_eq!(processed2, 1, "entry duplikat tetap ter-ack (bukan macet)");
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "handler TIDAK boleh terpanggil lagi untuk job_id yang sama (idempotent)"
    );

    flush_test_keys().await;

    // ── 4: handler selalu gagal → DLQ ───────────────────────────────────────
    let mut fail_registry = SchedulerRegistry::new();
    fail_registry.register(
        "fail_test",
        Arc::new(AlwaysFailHandler) as Arc<dyn JobHandler>,
    );

    let fail_client = SchedulerClient::new(Some(TEST_REDIS_URL.to_string()));
    let fail_env = JobEnvelope::new(
        "fail_test",
        serde_json::json!({}),
        chrono::Utc::now() - chrono::Duration::seconds(1),
    );
    fail_client
        .schedule(&fail_env)
        .await
        .expect("schedule harus sukses");

    let fail_consumer = SchedulerConsumer::new(TEST_REDIS_URL, fail_registry, "test-consumer")
        .expect("consumer harus terbentuk");
    fail_consumer
        .ensure_consumer_group()
        .await
        .expect("ensure group");
    fail_consumer.run_mover_tick().await.expect("mover tick");

    // process_one_batch berisi retry loop internal (dengan backoff) — satu panggilan
    // ini sudah mencakup seluruh percobaan sampai job masuk DLQ.
    let fail_processed = fail_consumer
        .process_one_batch()
        .await
        .expect("process batch");
    assert_eq!(fail_processed, 1);

    let redis_client = redis::Client::open(TEST_REDIS_URL).unwrap();
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let dlq_len: i64 = conn.xlen(DLQ_STREAM_KEY).await.unwrap();
    assert_eq!(
        dlq_len, 1,
        "job yang selalu gagal harus masuk DLQ tepat 1 entry"
    );

    let pending: redis::streams::StreamPendingCountReply = conn
        .xpending_count(STREAM_KEY, common_scheduler::CONSUMER_GROUP, "-", "+", 10)
        .await
        .unwrap();
    assert!(
        pending.ids.is_empty(),
        "entry asli harus sudah ter-ack (tidak macet di PEL) setelah masuk DLQ"
    );

    flush_test_keys().await;
}
