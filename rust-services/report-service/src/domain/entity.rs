use chrono::{DateTime, Utc};
use report_service_client::{ReportAdType, ReportStatus, ReportTargetType, ReportType};
use uuid::Uuid;

/// Entitas domain Report — representasi aduan di schema `report`.
#[derive(Debug, Clone)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Uuid,
    /// Jalur pembuatan aduan (P1.1/P1.4) — BUKAN `target_type` (target yang diadukan).
    pub report_type: ReportType,
    /// `None` untuk "Pelaporan Masalah" tanpa iklan terkait (PRD §6.10).
    pub target_type: Option<ReportTargetType>,
    pub target_id: Option<Uuid>,
    /// Jenis iklan yang diadukan (P9.0, Kelompok 6 Q9) — `None` bila
    /// `target_type=User` atau tidak diketahui.
    pub target_ad_type: Option<ReportAdType>,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    pub status: ReportStatus,
    pub action_note: Option<String>,
    pub reviewed_by: Option<Uuid>,
    /// Batas SLA 7 hari kerja (P1.2), dihitung sekali saat insert.
    pub due_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Report {
    /// `is_overdue` SENGAJA tidak dipersist (lihat migration) — dihitung di sini.
    pub fn is_overdue(&self) -> bool {
        !self.status.is_terminal() && self.due_date < Utc::now()
    }
}
