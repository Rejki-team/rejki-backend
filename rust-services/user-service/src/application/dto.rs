use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::RekeningInfo;

// ── Profile response (diperluas dengan status KYC + enkripsi at-rest) ─────

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    /// Peran pengguna (RBAC). Dipakai dashboard untuk header role.
    pub role: Option<String>,
    /// NIK ditampilkan ter-mask (hanya 4 digit terakhir)
    pub nik_masked: Option<String>,
    /// Status KYC terkini (pending / approved / rejected / null bila belum kirim)
    pub kyc_status: Option<String>,
    // ── Enkripsi at-rest (W3C-09) ─────────────────────────────────────
    /// Nama bank (plaintext dari rekening terdekripsi).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rekening_bank: Option<String>,
    /// Nomor rekening ter-mask (****1234).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rekening_masked: Option<String>,
    /// Nama pemilik rekening ter-mask (J***e).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rekening_holder_masked: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileInput {
    #[validate(length(max = 100))]
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    #[validate(length(max = 300))]
    pub bio: Option<String>,
    pub phone: Option<String>,
    /// Informasi rekening bank — akan di-encrypt AES-256-GCM sebelum disimpan.
    /// Validasi: bank non-empty, number digit-only, holder non-empty dilakukan di service layer.
    pub rekening: Option<RekeningInfo>,
}

// ── KYC data diri (US-04) ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct KycPersonalDataInput {
    #[validate(length(min = 1, max = 200))]
    pub full_name: String,
    /// NIK 16 digit — divalidasi panjang via derive, digit-only dicek manual di service (H1).
    #[validate(length(min = 16, max = 16, message = "NIK harus 16 digit"))]
    pub nik: String,
    #[validate(length(min = 1))]
    pub education_level: String,
    #[validate(length(min = 1))]
    pub gender: String,
    pub birth_date: chrono::NaiveDate,
    #[validate(length(min = 1))]
    pub address_line: String,
    #[serde(default = "default_country")]
    pub country_code: String,
    #[validate(length(min = 1))]
    pub province_id: String,
    #[validate(length(min = 1))]
    pub regency_id: String,
    #[validate(length(min = 1))]
    pub district_id: String,
    #[validate(length(min = 1))]
    pub village_id: String,
}

fn default_country() -> String {
    "ID".into()
}

// ── Avatar request ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct AvatarRequest {
    #[validate(length(min = 1))]
    pub mime: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct UploadPermission {
    pub presigned_url: String,
    pub object_key: String,
}

// ── Dokumen KYC (US-04) ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct DocumentRequest {
    /// "ktp" | "selfie"
    #[validate(length(min = 1))]
    pub kind: String,
    #[validate(length(min = 1))]
    pub mime: String,
    pub size_bytes: u64,
}

/// Commit dokumen: klien mengirim ulang header yang sudah diverifikasi magic bytes-nya.
#[derive(Debug, Deserialize, Validate)]
pub struct CommitDocumentInput {
    /// "ktp" | "selfie"
    #[validate(length(min = 1))]
    pub kind: String,
    /// Object key yang diterima saat permintaan upload (presigned).
    #[validate(length(min = 1))]
    pub object_key: String,
    /// MIME yang diklaim (harus cocok dengan magic_bytes).
    #[validate(length(min = 1))]
    pub mime: String,
    /// Header berkas (N byte pertama) dalam base64 — diverifikasi magic bytes.
    #[validate(length(min = 1))]
    pub magic_head_b64: String,
}

// ── KYC submission ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct KycSubmissionResponse {
    pub id: Uuid,
    pub status: String,
    pub review_note: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

// ── Review admin (placeholder — RBAC menyusul) ─────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct ReviewInput {
    #[serde(default)]
    pub approved: bool,
    /// Wajib bila menolak
    pub review_note: Option<String>,
}

// ── Admin: listing & detail pengajuan KYC (add-user-admin-management) ────────

/// Query string listing admin KYC. Mengikuti pola `{ q, status, sort_by, sort_dir,
/// limit, offset }` yang dipakai service lain agar konsisten di seluruh dashboard.
#[derive(Debug, Default, Deserialize)]
pub struct AdminKycListQuery {
    /// Kata kunci pencarian (Nama / ID submission / ID profil).
    pub q: Option<String>,
    /// Filter status verifikasi (pending / approved / rejected). Default: pending.
    pub status: Option<String>,
    /// Kolom urut (saat ini hanya `created_at`; disediakan untuk kompatibilitas UI).
    pub sort_by: Option<String>,
    /// Arah urut: `asc` | `desc` (default desc).
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Satu baris daftar pengajuan KYC untuk admin.
/// NIK TIDAK pernah disertakan penuh — hanya `nik_masked` (xxx...1234).
#[derive(Debug, Serialize)]
pub struct AdminKycListItem {
    /// ID pengajuan (submission), dipakai untuk membuka detail & dokumen.
    pub id: Uuid,
    pub full_name: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    pub nik_masked: Option<String>,
    pub status: String,
    pub created_at: String,
}

/// Detail satu pengajuan KYC untuk pop-up admin. NIK tetap ter-mask.
/// Menambah penanda ketersediaan dokumen agar UI dapat menampilkan click-to-view.
#[derive(Debug, Serialize)]
pub struct AdminKycDetail {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub full_name: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    pub nik_masked: Option<String>,
    pub status: String,
    /// Apakah dokumen KTP tersedia (belum dimusnahkan) — untuk click-to-view.
    pub has_ktp: bool,
    /// Apakah dokumen Selfie tersedia.
    pub has_selfie: bool,
    pub created_at: String,
}
