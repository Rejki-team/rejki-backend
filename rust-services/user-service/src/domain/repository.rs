use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::entity::{
    DocumentAccessAction, KycSubmission, KycSubmissionStatus, RekeningInfo, UserProfile,
};

/// Transaction handle — bungkus operasi tulis multi-step agar atomic (C2).
/// Hanya untuk operasi yang perlu rollback pada partial failure.
#[async_trait]
pub trait TxUserRepository: Send {
    async fn find_by_auth_id(
        &mut self,
        auth_id: Uuid,
    ) -> Result<Option<UserProfile>, anyhow::Error>;
    async fn update_profile(&mut self, profile: &UserProfile) -> Result<bool, anyhow::Error>;
    async fn create_submission(&mut self, profile_id: Uuid)
        -> Result<KycSubmission, anyhow::Error>;
    async fn commit(self: Box<Self>) -> Result<(), anyhow::Error>;
    async fn rollback(self: Box<Self>) -> Result<(), anyhow::Error>;
}

/// Parameter listing pengajuan KYC untuk admin — di-group agar menghindari
/// too-many-arguments (add-user-admin-management). `limit`/`offset` sudah ter-resolve.
#[derive(Debug, Clone)]
pub struct AdminKycListParams {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Hasil listing admin: baris + total (dari `COUNT(*) OVER()`, satu round-trip).
#[derive(Debug, Clone)]
pub struct AdminKycListResult {
    pub items: Vec<AdminKycRow>,
    pub total: i64,
}

/// Baris gabungan submission ⋈ profile untuk tampilan admin.
/// NIK TIDAK pernah dibawa penuh — hanya `nik_last4` untuk masking di service.
#[derive(Debug, Clone)]
pub struct AdminKycRow {
    pub submission_id: Uuid,
    pub status: KycSubmissionStatus,
    pub created_at: DateTime<Utc>,
    pub profile_id: Uuid,
    pub full_name: Option<String>,
    pub nik_last4: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    pub ktp_object_key: Option<String>,
    pub selfie_object_key: Option<String>,
}

/// Parameter update profil — refactored dari positional args untuk menghindari
/// clippy::too_many_arguments (W3C-09).
#[derive(Debug, Clone, Default)]
pub struct UpdateProfileParams {
    pub id: Uuid,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    /// Rekening — akan di-serialize ke JSON lalu di-encrypt AES-256-GCM di repo layer.
    pub rekening: Option<RekeningInfo>,
    // ── Region fields (W3C-11) ────────────────────────────────────────────
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
}

// Native async fn in trait per CLAUDE.md §4.1 (Rust ≥ 1.75).
// Trait TIDAK digunakan sebagai dyn object, jadi async-trait tidak diperlukan.
#[allow(async_fn_in_trait)]
pub trait UserRepository: Send + Sync {
    // Profil
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    async fn find_by_auth_id(&self, auth_id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    async fn create(&self, auth_id: Uuid, username: &str) -> Result<UserProfile, anyhow::Error>;
    /// Update profil — gunakan `UpdateProfileParams` untuk menghindari too_many_arguments.
    async fn update(&self, params: UpdateProfileParams) -> Result<UserProfile, anyhow::Error>;
    async fn update_avatar(&self, id: Uuid, object_key: &str) -> Result<(), anyhow::Error>;
    /// Perbarui SEMUA field profil (termasuk KYC) — dipakai untuk simpan data diri.
    /// Atomic: UPDATE hanya bila `nik_encrypted IS NULL` (NIK guard, anti-race C1).
    /// Returns `true` bila baris terupdate, `false` bila NIK sudah ada (immutable).
    async fn update_profile(&self, profile: &UserProfile) -> Result<bool, anyhow::Error>;

    // KYC submission
    async fn create_submission(&self, profile_id: Uuid) -> Result<KycSubmission, anyhow::Error>;
    async fn get_submission_by_id(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<KycSubmission>, anyhow::Error>;
    async fn get_latest_submission(
        &self,
        profile_id: Uuid,
    ) -> Result<Option<KycSubmission>, anyhow::Error>;
    /// Atomically update submission status if currently pending.
    /// Returns true if a row was updated, false if already terminal (race-safe).
    async fn review_submission(
        &self,
        id: Uuid,
        status: KycSubmissionStatus,
        reviewed_by: Uuid,
        review_note: Option<&str>,
    ) -> Result<bool, anyhow::Error>;

    /// Set object key dokumen (`kind` = "ktp" | "selfie") pada submission (commit).
    async fn set_document_key(
        &self,
        submission_id: Uuid,
        kind: &str,
        object_key: &str,
    ) -> Result<(), anyhow::Error>;

    /// Kosongkan object key dokumen pada submission (pemusnahan, retensi K11).
    async fn clear_document_keys(&self, submission_id: Uuid) -> Result<(), anyhow::Error>;

    // Audit trail dokumen (Q2) — append-only.
    async fn log_document_access(
        &self,
        actor_id: Uuid,
        object_key: &str,
        action: DocumentAccessAction,
        request_id: Option<&str>,
    ) -> Result<(), anyhow::Error>;

    // ── Admin listing KYC (add-user-admin-management) ───────────────────────────

    /// Daftar pengajuan KYC untuk admin (filter/sort/pagination) + total.
    async fn admin_list_submissions(
        &self,
        params: AdminKycListParams,
    ) -> Result<AdminKycListResult, anyhow::Error>;

    /// Daftar pengajuan KYC tanpa offset (untuk ekspor CSV), dibatasi `limit`.
    async fn admin_list_submissions_all(
        &self,
        params: AdminKycListParams,
    ) -> Result<Vec<AdminKycRow>, anyhow::Error>;

    /// Detail satu pengajuan (submission ⋈ profile) by submission id.
    async fn get_submission_with_profile(
        &self,
        submission_id: Uuid,
    ) -> Result<Option<AdminKycRow>, anyhow::Error>;

    /// Mulai database transaction untuk operasi multi-step atomic (C2).
    /// Return `TxUserRepository` handle; panggil `commit()` atau `rollback()`.
    async fn begin_transaction(&self) -> Result<Box<dyn TxUserRepository>, anyhow::Error>;
}
