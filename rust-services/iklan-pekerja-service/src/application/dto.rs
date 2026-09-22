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
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
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
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Detail satu iklan pekerja untuk pop-up admin (F-27b) — sama seperti
/// `AdminIklanPekerjaResponse` ditambah indikator dokumen sensitif milik poster
/// (NIK/KTP/Selfie, di-resolve lewat `UserClient` ke user-service, TANPA
/// menyalin data sensitif ke schema `iklan-pekerja-service`).
#[derive(Debug, Serialize)]
pub struct AdminIklanPekerjaDetailResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub has_nik: bool,
    pub has_ktp: bool,
    pub has_selfie: bool,
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
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
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
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
    pub foto_urls: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Koordinat pengguna (F-1) — filter radius aktif hanya bila `latitude`+`longitude` diisi
    /// keduanya. Nama key sesuai kontrak mobile existing (`job_query_params.dart`/
    /// `worker_remote_datasource.dart`: `latitude`/`longitude`).
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// Override radius default (km) — mobile Iklan Pekerja sudah mengirim ini
    /// (`worker_remote_datasource.dart`: `max_distance`).
    pub max_distance: Option<f64>,
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
