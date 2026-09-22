//! `JobHandler` untuk tugas otomatis Bab 10 PRD milik Iklan Pelatihan (P6.4, P6.6-P6.8,
//! Kelompok 2 Phase 6, F-32). Dijadwalkan oleh `IklanPelatihanService` (lihat
//! `schedule_pelatihan_lifecycle_jobs`/`create_enrollment`), dieksekusi oleh
//! `common_scheduler::SchedulerConsumer` yang di-wiring di composition root (`rejki-app`).
//!
//! Setiap handler mengecek ULANG kondisi terkini sebelum bertindak (idempoten) —
//! penjadwalan hanya berarti "cek nanti pada waktunya", bukan "pasti eksekusi".
//! Logging terstruktur (job_type/entity_id/hasil) di setiap handler memenuhi P6.14
//! ("wajib tercatat pada log sistem agar dapat ditelusuri bila terjadi sengketa").

use std::sync::Arc;

use serde::Deserialize;
use uuid::Uuid;

use auth_service_client::AuthClient;
use common_scheduler::JobHandler;

use crate::domain::entity::PelatihanStatus;
use crate::domain::repository::IklanPelatihanRepository;
use crate::infrastructure::pg_repository::PgIklanPelatihanRepository;

pub const JOB_CANCEL_UNVERIFIED: &str = "pelatihan_cancel_unverified";
pub const JOB_MARK_SELESAI: &str = "pelatihan_mark_selesai";
pub const JOB_CHECK_BADGE_DEADLINE: &str = "pelatihan_check_badge_deadline";
pub const JOB_CANCEL_ENROLLMENT_UNPAID: &str = "pelatihan_cancel_enrollment_unpaid";

/// Suspensi organizer (P6.7) — 7x24 jam, sesuai Bab 10 PRD.
const ORGANIZER_SUSPEND_DAYS: i64 = 7;

#[derive(Deserialize)]
struct PelatihanIdPayload {
    pelatihan_id: Uuid,
}

#[derive(Deserialize)]
struct EnrollmentIdPayload {
    enrollment_id: Uuid,
}

/// P6.4: H-8 jam sebelum mulai, masih dalam status verifikasi → batalkan otomatis.
pub struct CancelUnverifiedHandler {
    pub repo: Arc<PgIklanPelatihanRepository>,
}

#[async_trait::async_trait]
impl JobHandler for CancelUnverifiedHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: PelatihanIdPayload = serde_json::from_value(payload.clone())?;
        let Some(pelatihan) = self.repo.find_by_id(p.pelatihan_id).await? else {
            tracing::warn!(job_type = JOB_CANCEL_UNVERIFIED, pelatihan_id = %p.pelatihan_id, "entity tidak ditemukan — skip");
            return Ok(());
        };
        if pelatihan.deleted_at.is_some()
            || !matches!(
                pelatihan.status,
                PelatihanStatus::VerifikasiTertunda | PelatihanStatus::VerifikasiDalamProses
            )
        {
            tracing::info!(
                job_type = JOB_CANCEL_UNVERIFIED,
                pelatihan_id = %p.pelatihan_id,
                status = %pelatihan.status,
                hasil = "no_op_sudah_diverifikasi_atau_dihapus",
                "tidak perlu dibatalkan"
            );
            return Ok(());
        }

        let changed = self
            .repo
            .set_pelatihan_status_system(p.pelatihan_id, PelatihanStatus::Dibatalkan.as_str())
            .await?;
        tracing::warn!(
            job_type = JOB_CANCEL_UNVERIFIED,
            pelatihan_id = %p.pelatihan_id,
            hasil = if changed { "dibatalkan" } else { "no_op_race_condition" },
            "pelatihan dibatalkan otomatis (H-8 jam sebelum mulai, masih verifikasi)"
        );
        Ok(())
    }
}

/// P6.6: 1x24 jam setelah berakhir, belum ditekan Akhiri → tandai Pelatihan Selesai.
pub struct MarkSelesaiHandler {
    pub repo: Arc<PgIklanPelatihanRepository>,
}

#[async_trait::async_trait]
impl JobHandler for MarkSelesaiHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: PelatihanIdPayload = serde_json::from_value(payload.clone())?;
        let Some(pelatihan) = self.repo.find_by_id(p.pelatihan_id).await? else {
            tracing::warn!(job_type = JOB_MARK_SELESAI, pelatihan_id = %p.pelatihan_id, "entity tidak ditemukan — skip");
            return Ok(());
        };
        if pelatihan.deleted_at.is_some() || pelatihan.status.is_terminal() {
            tracing::info!(
                job_type = JOB_MARK_SELESAI,
                pelatihan_id = %p.pelatihan_id,
                status = %pelatihan.status,
                hasil = "no_op_sudah_terminal_atau_dihapus",
                "tidak perlu ditandai selesai"
            );
            return Ok(());
        }

        let changed = self
            .repo
            .set_pelatihan_status_system(p.pelatihan_id, PelatihanStatus::PelatihanSelesai.as_str())
            .await?;
        tracing::info!(
            job_type = JOB_MARK_SELESAI,
            pelatihan_id = %p.pelatihan_id,
            hasil = if changed { "ditandai_selesai" } else { "no_op_race_condition" },
            "pelatihan ditandai selesai otomatis (1x24 jam setelah tanggal_selesai, admin belum menekan Akhiri)"
        );
        Ok(())
    }
}

/// P6.7: 3x24 jam setelah selesai, badge/sertifikat belum diajukan → suspend
/// penyelenggara 7x24 jam (`iklan-pelatihan-service` -> `AuthClient`).
pub struct CheckBadgeDeadlineHandler {
    pub repo: Arc<PgIklanPelatihanRepository>,
    pub auth_client: Arc<dyn AuthClient>,
}

#[async_trait::async_trait]
impl JobHandler for CheckBadgeDeadlineHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: PelatihanIdPayload = serde_json::from_value(payload.clone())?;
        let Some(pelatihan) = self.repo.find_by_id(p.pelatihan_id).await? else {
            tracing::warn!(job_type = JOB_CHECK_BADGE_DEADLINE, pelatihan_id = %p.pelatihan_id, "entity tidak ditemukan — skip");
            return Ok(());
        };
        if pelatihan.deleted_at.is_some() {
            tracing::info!(job_type = JOB_CHECK_BADGE_DEADLINE, pelatihan_id = %p.pelatihan_id, hasil = "no_op_dihapus", "pelatihan sudah dihapus");
            return Ok(());
        }
        if self
            .repo
            .has_any_badge_for_pelatihan(p.pelatihan_id)
            .await?
        {
            tracing::info!(
                job_type = JOB_CHECK_BADGE_DEADLINE,
                pelatihan_id = %p.pelatihan_id,
                hasil = "no_op_badge_sudah_diajukan",
                "badge/sertifikat sudah diajukan — penyelenggara tidak disuspend"
            );
            return Ok(());
        }

        self.auth_client
            .suspend_temporarily(
                pelatihan.poster_id,
                ORGANIZER_SUSPEND_DAYS,
                "Bab 10 PRD: badge/sertifikat pelatihan tidak diajukan dalam 3x24 jam setelah pelatihan selesai",
            )
            .await
            .map_err(|e| anyhow::anyhow!("gagal suspend penyelenggara: {e}"))?;
        tracing::warn!(
            job_type = JOB_CHECK_BADGE_DEADLINE,
            pelatihan_id = %p.pelatihan_id,
            poster_id = %pelatihan.poster_id,
            hasil = "penyelenggara_disuspend_7x24_jam",
            "penyelenggara disuspend otomatis — badge/sertifikat tidak diajukan dalam 3x24 jam"
        );
        Ok(())
    }
}

/// P6.8: 1x24 jam setelah kode pembayaran terbit (= enrollment dibuat), bukti transfer
/// belum dikirim → batalkan pendaftaran.
pub struct CancelEnrollmentUnpaidHandler {
    pub repo: Arc<PgIklanPelatihanRepository>,
}

#[async_trait::async_trait]
impl JobHandler for CancelEnrollmentUnpaidHandler {
    async fn handle(&self, payload: &serde_json::Value) -> Result<(), anyhow::Error> {
        let p: EnrollmentIdPayload = serde_json::from_value(payload.clone())?;
        let Some(enrollment) = self.repo.find_enrollment_by_id(p.enrollment_id).await? else {
            tracing::warn!(job_type = JOB_CANCEL_ENROLLMENT_UNPAID, enrollment_id = %p.enrollment_id, "entity tidak ditemukan — skip");
            return Ok(());
        };
        if enrollment.bukti_transfer_object_key.is_some() || !enrollment.status.can_review() {
            tracing::info!(
                job_type = JOB_CANCEL_ENROLLMENT_UNPAID,
                enrollment_id = %p.enrollment_id,
                status = %enrollment.status,
                hasil = "no_op_bukti_sudah_dikirim_atau_sudah_direview",
                "tidak perlu dibatalkan"
            );
            return Ok(());
        }

        let updated = self
            .repo
            .review_enrollment(
                p.enrollment_id,
                false,
                Some("Dibatalkan otomatis: bukti pembayaran tidak dikirim dalam 24 jam sejak kode pembayaran terbit (Bab 10 PRD)"),
                Uuid::nil(), // sentinel "sistem" — dipicu scheduler, bukan admin manusia
            )
            .await?;
        tracing::warn!(
            job_type = JOB_CANCEL_ENROLLMENT_UNPAID,
            enrollment_id = %p.enrollment_id,
            hasil = if updated.is_some() { "dibatalkan" } else { "no_op_race_condition" },
            "pendaftaran dibatalkan otomatis — bukti pembayaran tidak dikirim dalam 24 jam"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_constants_are_unique() {
        let names = [
            JOB_CANCEL_UNVERIFIED,
            JOB_MARK_SELESAI,
            JOB_CHECK_BADGE_DEADLINE,
            JOB_CANCEL_ENROLLMENT_UNPAID,
        ];
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "job_type harus unik: {names:?}");
    }
}
