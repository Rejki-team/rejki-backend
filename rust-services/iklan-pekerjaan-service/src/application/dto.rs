use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::ModerationStatus;

#[derive(Debug, Serialize)]
pub struct IklanPekerjaanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub created_at: DateTime<Utc>,
}

/// Admin response – includes deleted_at for moderation context.
#[derive(Debug, Serialize)]
pub struct AdminIklanPekerjaanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPekerjaanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 2, max = 200))]
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub foto_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Admin query ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuspendEvidenceInput {
    pub mime: String,
    #[validate(range(min = 1, max = 5242880))]
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuspendInput {
    pub iklan_ids: Vec<Uuid>,
    pub is_permanent: bool,
    #[validate(length(min = 10))]
    pub reason: String,
    pub evidence_object_key: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SuspendResultItem {
    pub iklan_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuspendResponse {
    pub results: Vec<SuspendResultItem>,
}
