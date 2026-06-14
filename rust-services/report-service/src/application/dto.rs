use chrono::{DateTime, Utc};
use report_service_client::{ReportStatus, ReportTargetType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ══════════════════════════════════════════════════════════════════════════
// Response
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct ReportResponse {
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

/// Detail aduan untuk admin — termasuk presigned read URL bukti.
#[derive(Debug, Serialize)]
pub struct ReportDetailResponse {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_read_url: Option<String>,
    pub status: ReportStatus,
    pub action_note: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════════
// Inputs
// ══════════════════════════════════════════════════════════════════════════

/// Input pembuatan aduan dari sisi mobile.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateReportInput {
    #[validate(length(min = 1, message = "target_type wajib diisi"))]
    pub target_type: String,
    pub target_id: Uuid,
    #[validate(length(
        min = 1,
        max = 2000,
        message = "keterangan wajib diisi (maks 2000 karakter)"
    ))]
    pub keterangan: String,
    pub mime: Option<String>,
    pub size_bytes: Option<u64>,
}

/// Query params untuk listing admin.
#[derive(Debug, Deserialize)]
pub struct ReportListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Input tindak lanjut admin.
#[derive(Debug, Deserialize, Validate)]
pub struct ReviewReportInput {
    pub approved: bool,
    #[validate(length(
        min = 1,
        message = "action_note wajib diisi — jelaskan tindakan yang diambil"
    ))]
    pub action_note: String,
}
