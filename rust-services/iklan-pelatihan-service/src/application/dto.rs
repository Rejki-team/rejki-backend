use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::{CreatedByRole, EnrollmentStatus, ModerationStatus, PelatihanStatus};

// ══════════════════════════════════════════════════════════════════════════
// Pelatihan responses
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct IklanPelatihanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub status: PelatihanStatus,
    pub created_by_role: CreatedByRole,
    pub jumlah_peserta: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminIklanPelatihanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub status: PelatihanStatus,
    pub created_by_role: CreatedByRole,
    pub jumlah_peserta: Option<i32>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════════
// Pelatihan inputs
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPelatihanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 1))]
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub foto_urls: Option<Vec<String>>,
    pub jumlah_peserta: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateIklanPelatihanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 1))]
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub jumlah_peserta: Option<i32>,
}

#[derive(Debug, Clone, Validate, Deserialize)]
pub struct UpdatePelatihanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: Option<String>,
    #[validate(length(min = 1))]
    pub penyelenggara: Option<String>,
    #[validate(length(min = 1))]
    pub deskripsi: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub foto_urls: Option<Vec<String>>,
    pub jumlah_peserta: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewPelatihanInput {
    pub approved: bool,
    pub review_note: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════════
// Query params
// ══════════════════════════════════════════════════════════════════════════

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

#[derive(Debug, Deserialize)]
pub struct PelatihanListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ══════════════════════════════════════════════════════════════════════════
// Enrollment
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct EnrollmentResponse {
    pub id: Uuid,
    pub pelatihan_id: Uuid,
    pub user_id: Uuid,
    pub bukti_transfer_object_key: Option<String>,
    pub status: EnrollmentStatus,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct EnrollEvidenceInput {
    pub mime: String,
    #[validate(range(min = 1, max = 5242880))]
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct CommitEnrollBuktiInput {
    pub object_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewEnrollmentInput {
    pub approved: bool,
    pub review_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollmentListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ══════════════════════════════════════════════════════════════════════════
// Badge
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct BadgeResponse {
    pub id: Uuid,
    pub pelatihan_id: Uuid,
    pub user_id: Uuid,
    pub sertifikat_object_key: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub status: EnrollmentStatus,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BadgeEvidenceInput {
    pub mime: String,
    #[validate(range(min = 1, max = 10485760))]
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct CommitBadgeSertifikatInput {
    pub object_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewBadgeInput {
    pub approved: bool,
    pub review_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BadgeListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ══════════════════════════════════════════════════════════════════════════
// Suspension (existing)
// ══════════════════════════════════════════════════════════════════════════

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
