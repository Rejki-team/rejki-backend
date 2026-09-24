//! `JobHandler` untuk tugas otomatis Bab 10 PRD milik Chat (P4.3, P4.5, Kelompok 4
//! Phase 4, F-19). Dijadwalkan oleh `ChatService` (lihat `get_or_create_conversation`
//! untuk retensi, `schedule_auto_end_for_ad` untuk auto-end), dieksekusi oleh
//! `common_scheduler::SchedulerConsumer` yang di-wiring di composition root (`rejki-app`).
//!
//! Setiap handler mengecek ULANG kondisi terkini sebelum bertindak (idempoten) —
//! penjadwalan hanya berarti "cek nanti pada waktunya", bukan "pasti eksekusi".

use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use common_scheduler::JobHandler;

use crate::domain::repository::ChatRepository;
use crate::infrastructure::pg_repository::PgChatRepository;

pub const JOB_CHAT_AUTO_END: &str = "chat_auto_end_conversation";
pub const JOB_CHAT_RETENTION_PURGE: &str = "chat_retention_purge";

#[derive(Deserialize)]
struct ConversationIdPayload {
    conversation_id: Uuid,
}

/// P4.3: 2x24 jam setelah "proses pada iklan terkait selesai" → tandai `ended_at`.
pub struct AutoEndHandler {
    pub repo: Arc<PgChatRepository>,
}

#[async_trait::async_trait]
impl JobHandler for AutoEndHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: ConversationIdPayload = serde_json::from_value(payload.clone())?;
        let changed = self.repo.auto_end_conversation(p.conversation_id).await?;
        tracing::info!(
            job_type = JOB_CHAT_AUTO_END,
            conversation_id = %p.conversation_id,
            hasil = if changed { "diakhiri_otomatis" } else { "no_op_sudah_berakhir_atau_dihapus" },
            "auto-end percakapan (2x24 jam setelah proses iklan terkait selesai)"
        );
        Ok(())
    }
}

/// P4.5: 60 hari setelah conversation dibuat → hapus riwayat percakapan (hard delete,
/// bukan soft-delete — PRD eksplisit "dihapus"), tidak bergantung status `ended_at`.
pub struct RetentionPurgeHandler {
    pub repo: Arc<PgChatRepository>,
}

#[async_trait::async_trait]
impl JobHandler for RetentionPurgeHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: ConversationIdPayload = serde_json::from_value(payload.clone())?;
        let purged = self.repo.purge_conversation(p.conversation_id).await?;
        tracing::warn!(
            job_type = JOB_CHAT_RETENTION_PURGE,
            conversation_id = %p.conversation_id,
            hasil = if purged { "dihapus" } else { "no_op_sudah_dihapus" },
            "retensi 60 hari — riwayat percakapan dihapus otomatis"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_constants_are_unique() {
        let names = [JOB_CHAT_AUTO_END, JOB_CHAT_RETENTION_PURGE];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "job_type harus unik: {names:?}");
    }
}
