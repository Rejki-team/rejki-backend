use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::ModerationStatus;

#[derive(Debug, Serialize)]
pub struct IklanPekerjaResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminIklanPekerjaResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPekerjaInput {
    #[validate(length(min = 2, max = 200))]
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub foto_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Validate, Deserialize)]
pub struct UpdatePekerjaInput {
    #[validate(length(min = 2, max = 200))]
    pub nama: Option<String>,
    pub keahlian: Option<Vec<String>>,
    #[validate(length(min = 1))]
    pub deskripsi: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub foto_urls: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

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
