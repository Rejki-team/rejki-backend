use chrono::{DateTime, Utc};
use report_service_client::{ReportStatus, ReportTargetType};
use uuid::Uuid;

/// Entitas domain Report — representasi aduan di schema `report`.
#[derive(Debug, Clone)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    pub status: ReportStatus,
    pub action_note: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
