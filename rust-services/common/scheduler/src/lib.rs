//! Scheduler generik berbasis Redis Streams (F-32, PRD Bab 10) — dipakai untuk
//! seluruh tugas otomatis berjadwal (mis. tutup pendaftaran pelatihan H-30 menit,
//! hapus dokumen KYC setelah ditolak, dst. — lihat Kelompok 2 Phase 6).
//!
//! ## Arsitektur (keputusan teknis P5.1, tanpa preseden di codebase — didokumentasikan
//! di sini sesuai izin plan §4 Risks)
//!
//! Redis Streams sendiri TIDAK punya mekanisme "invisible until time T" (tidak seperti
//! SQS delay queue). Dua opsi dievaluasi:
//! 1. **Sorted-set delay buffer** (dipilih): `SchedulerClient::schedule()` menyimpan job
//!    di sorted-set (`ZADD`, score = unix timestamp `execute_after`). Background "mover"
//!    (`SchedulerConsumer::run_mover_tick`, dipanggil berkala dari `run()`) memindahkan
//!    job yang jatuh tempo (`ZRANGEBYSCORE ... <= now`) ke stream via **satu Lua script
//!    atomik** (`ZREM` + `XADD` per job, dieksekusi Redis single-threaded — aman
//!    multi-instance TANPA lock tambahan, lihat Hazard #1).
//! 2. **`XADD` langsung + consumer cek `execute_after`** (ditolak): consumer harus
//!    "menahan" entry yang belum jatuh tempo tanpa `XACK`, lalu re-check berkala — Redis
//!    Streams tidak punya cara native men-"unassign" entry yang sudah masuk PEL consumer
//!    tanpa idle-time timeout (`XAUTOCLAIM`), sehingga pola ini butuh polling/timer
//!    tambahan yang justru MENDUPLIKASI apa yang sudah didapat gratis dari opsi 1.
//!
//! Setelah masuk stream, pemrosesan mengikuti **persis** pola `bun-notification-service`
//! yang sudah teruji (P5.2): `XREADGROUP` per consumer-group (Hazard #1 — satu entry
//! hanya dikirim ke SATU consumer dalam grup, no lock tambahan diperlukan), retry
//! in-process dengan exponential backoff+jitter (§4.5 backend, lebih ketat dari referensi
//! TS-nya) maks `MAX_RETRY` kali, lalu `XADD` ke stream `_dlq` + `XACK` entry asli.
//! `XAUTOCLAIM` saat startup memulihkan job yang macet di consumer yang crash.
//!
//! Idempotency: marker `scheduler:processed:<job_id>` (Redis `SET NX EX`, TTL 7 hari) —
//! DICEK SEBELUM handler dipanggil (bukan hanya audit trail seperti referensi TS-nya) —
//! supaya setiap `JobHandler` yang didaftarkan TIDAK perlu membangun idempotency sendiri.

mod client;
mod consumer;
mod envelope;
mod error;
mod registry;

pub use client::SchedulerClient;
pub use consumer::SchedulerConsumer;
pub use envelope::JobEnvelope;
pub use error::SchedulerError;
pub use registry::{JobHandler, SchedulerRegistry};

use std::time::Duration;

/// Sorted-set delay buffer (score = unix timestamp detik `execute_after`).
pub const DELAYED_ZSET_KEY: &str = "scheduler:delayed";
/// Stream utama — konsumsi via consumer-group `CONSUMER_GROUP`.
pub const STREAM_KEY: &str = "scheduler:jobs";
/// Dead-letter stream — job yang gagal setelah `MAX_RETRY` percobaan.
pub const DLQ_STREAM_KEY: &str = "scheduler:jobs:dlq";
/// Nama consumer-group (semua instance `SchedulerConsumer` bergabung ke grup ini —
/// menjamin satu entry hanya diproses satu instance, Hazard #1).
pub const CONSUMER_GROUP: &str = "scheduler_workers";
/// Prefix key idempotency marker — `scheduler:processed:<job_id>`.
pub const IDEMPOTENCY_PREFIX: &str = "scheduler:processed:";
/// TTL marker idempotency (7 hari — sama dengan `bun-notification-service`).
pub const PROCESSED_TTL_SECS: u64 = 7 * 24 * 60 * 60;
/// Maks percobaan sebelum job masuk DLQ (§4.5 backend: "batas attempt").
pub const MAX_RETRY: u32 = 3;
/// Idle-time minimum (ms) sebelum entry PEL consumer lain boleh di-`XAUTOCLAIM`.
pub const RECLAIM_MIN_IDLE_MS: u64 = 60_000;
/// Jumlah maksimum job dipindah per `run_mover_tick` / diklaim per `XAUTOCLAIM`.
pub const MOVER_BATCH: i64 = 100;
/// Interval polling mover — seberapa sering delay buffer dicek untuk job jatuh tempo.
pub const MOVER_INTERVAL: Duration = Duration::from_secs(5);
/// Jumlah entry maksimum per `XREADGROUP`.
pub const BATCH_SIZE: i64 = 10;
/// `BLOCK` (ms) untuk `XREADGROUP` — berapa lama menunggu entry baru sebelum kembali
/// ke loop utama (memberi kesempatan mover tick berjalan meski stream sedang sepi).
pub const BLOCK_MS: u64 = 5_000;
