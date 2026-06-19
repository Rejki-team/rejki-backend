use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::{AvailabilityStatus, ModerationStatus};

/// Response publik — tanpa harga/kondisi, dengan kolom gratis.
#[derive(Debug, Serialize)]
pub struct IklanBarangBekasResponse {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub judul: String,
    pub deskripsi: String,
    pub jenis_barang: String,
    pub jumlah: i32,
    pub lokasi_pengambilan: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub foto_urls: Vec<String>,
    pub availability_status: AvailabilityStatus,
    pub moderation_status: ModerationStatus,
    pub created_at: DateTime<Utc>,
}

/// Response admin — kolom gratis + deleted_at/updated_at.
#[derive(Debug, Serialize)]
pub struct AdminIklanBarangBekasResponse {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub judul: String,
    pub deskripsi: String,
    pub jenis_barang: String,
    pub jumlah: i32,
    pub lokasi_pengambilan: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub foto_urls: Vec<String>,
    pub availability_status: AvailabilityStatus,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input create iklan barang gratis.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanBarangBekasInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    pub deskripsi: String,
    /// "bekas" | "baru"
    #[validate(custom(function = "validate_jenis_barang"))]
    pub jenis_barang: String,
    /// Jumlah barang (≥1)
    #[validate(range(min = 1))]
    pub jumlah: i32,
    /// Lokasi pengambilan (wajib)
    #[validate(length(min = 1))]
    pub lokasi_pengambilan: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub foto_urls: Option<Vec<String>>,
}

/// Input update iklan barang gratis — semua field Optional.
#[derive(Debug, Clone, Validate, Deserialize)]
pub struct UpdateBarangBekasInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: Option<String>,
    #[validate(length(min = 1))]
    pub deskripsi: Option<String>,
    pub jenis_barang: Option<String>, // "bekas" | "baru"
    #[validate(range(min = 1))]
    pub jumlah: Option<i32>,
    #[validate(length(min = 1))]
    pub lokasi_pengambilan: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
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

// ── Custom validators ────────────────────────────────────────────────────────

/// Validasi jenis_barang hanya menerima "bekas" atau "baru".
fn validate_jenis_barang(s: &str) -> Result<(), validator::ValidationError> {
    if s == "bekas" || s == "baru" {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("invalid_jenis_barang");
        err.message = Some(format!("jenis_barang harus 'bekas' atau 'baru', bukan '{s}'").into());
        Err(err)
    }
}
