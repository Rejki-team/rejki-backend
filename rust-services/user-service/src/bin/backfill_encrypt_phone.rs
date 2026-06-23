//! One-time backfill: enkripsi phone plaintext lama → `phone_encrypted`.
//!
//! # Cara pakai
//! ```bash
//! cargo run --package user-service --bin backfill_encrypt_phone
//! ```
//!
//! # Safety
//! - Idempoten: aman dijalankan berulang
//! - Setiap row: coba `common_crypto::decrypt(phone)` dulu
//!   — sukses → sudah terenkripsi, skip
//!   — gagal → `common_crypto::encrypt(phone)` → UPDATE `phone_encrypted`
//! - Tidak menyentuh data selain `phone_encrypted`
//! - Progress log tiap 100 baris + total di akhir
//! - Exit code 0 sukses, 1 error

use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Load .env (development) ──────────────────────────────────────
    dotenvy::dotenv().ok();

    // ── 2. Init tracing ─────────────────────────────────────────────────
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let db_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL wajib di-set — lihat .env.example");

    // Pastikan DATA_ENCRYPTION_KEY tersedia (akan dipakai common_crypto::encrypt)
    let _key_check = std::env::var("DATA_ENCRYPTION_KEY")
        .expect("DATA_ENCRYPTION_KEY wajib di-set — lihat .env.example");

    // ── 3. Connect ──────────────────────────────────────────────────────
    tracing::info!("connecting to database…");
    let pool = sqlx::PgPool::connect(&db_url).await?;
    tracing::info!("connected");

    // ── 4. Cari baris yang perlu backfill ───────────────────────────────
    let rows = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT id, phone FROM user_svc.profiles WHERE phone IS NOT NULL AND phone_encrypted IS NULL",
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();
    tracing::info!(total, "baris perlu backfill");

    if total == 0 {
        tracing::info!("tidak ada data yang perlu backfill — selesai");
        return Ok(());
    }

    // ── 5. Backfill ─────────────────────────────────────────────────────
    let start = Instant::now();
    let mut ok = 0u64;
    let mut skipped = 0u64;
    let mut failed = 0u64;

    for (i, (id, phone)) in rows.iter().enumerate() {
        // Coba decrypt dulu — kalau sukses berarti sudah terenkripsi
        if common_crypto::decrypt(phone).is_ok() {
            tracing::warn!(%id, "phone sudah terenkripsi — skip (seharusnya tidak muncul di query)");
            skipped += 1;
            continue;
        }

        match common_crypto::encrypt(phone) {
            Ok(enc) => {
                let updated = sqlx::query(
                    "UPDATE user_svc.profiles SET phone_encrypted = $1, updated_at = now() WHERE id = $2",
                )
                .bind(&enc)
                .bind(id)
                .execute(&pool)
                .await?;

                if updated.rows_affected() > 0 {
                    ok += 1;
                } else {
                    tracing::warn!(%id, "UPDATE tidak memengaruhi baris — mungkin dihapus");
                    skipped += 1;
                }
            }
            Err(e) => {
                tracing::error!(%id, error = %e, "enkripsi gagal");
                failed += 1;
            }
        }

        if (i + 1) % 100 == 0 {
            tracing::info!(processed = i + 1, total, "progress");
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(
        ok,
        skipped,
        failed,
        total,
        elapsed_ms = elapsed.as_millis() as u64,
        "backfill selesai"
    );

    if failed > 0 {
        Err(format!("{failed} baris gagal dienkripsi — lihat log di atas").into())
    } else {
        tracing::info!("✅ backfill selesai — semua sukses");
        Ok(())
    }
}
