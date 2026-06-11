use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ── Profile response (diperluas dengan status KYC) ──────────────────────────

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id:          Uuid,
    pub username:    String,
    pub full_name:   Option<String>,
    pub avatar:      Option<String>,
    pub bio:         Option<String>,
    pub phone:       Option<String>,
    /// NIK ditampilkan ter-mask (hanya 4 digit terakhir)
    pub nik_masked:  Option<String>,
    /// Status KYC terkini (pending / approved / rejected / null bila belum kirim)
    pub kyc_status:  Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileInput {
    #[validate(length(max = 100))]
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    #[validate(length(max = 300))]
    pub bio: Option<String>,
    pub phone: Option<String>,
}

// ── KYC data diri (US-04) ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct KycPersonalDataInput {
    #[validate(length(min = 1, max = 200))]
    pub full_name:    String,
    /// NIK 16 digit — divalidasi panjangnya; disimpan terenkripsi at-rest
    #[validate(length(min = 16, max = 16, message = "NIK harus 16 digit"))]
    pub nik:          String,
    #[validate(length(min = 1))]
    pub education_level: String,
    #[validate(length(min = 1))]
    pub gender:       String,
    pub birth_date:   chrono::NaiveDate,
    #[validate(length(min = 1))]
    pub address_line: String,
    #[serde(default = "default_country")]
    pub country_code: String,
    #[validate(length(min = 1))]
    pub province_id:  String,
    #[validate(length(min = 1))]
    pub regency_id:   String,
    #[validate(length(min = 1))]
    pub district_id:  String,
    #[validate(length(min = 1))]
    pub village_id:   String,
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

// ── KYC submission ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct KycSubmissionResponse {
    pub id:     Uuid,
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
