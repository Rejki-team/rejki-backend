use super::entity::{IklanPelatihan, IklanSuspension, PelatihanBadge, PelatihanEnrollment};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unified list/sort/search params untuk semua listing admin.
/// `filter_column` menentukan kolom mana yang difilter (moderation_status atau status).
#[derive(Debug, Clone)]
pub struct ListParams {
    pub q: Option<String>,
    pub filter_column: String,
    pub filter_value: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Untuk backward-compat dengan moderation listing.
#[derive(Debug, Clone)]
pub struct AdminListParams {
    pub q: Option<String>,
    pub moderation_status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

// Re-export legacy names as aliases for backward compat.
pub type PelatihanListParams = ListParams;
pub type EnrollmentListParams = ListParams;
pub type BadgeListParams = ListParams;

#[derive(Debug, Clone)]
pub struct AdminListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/// Default page size untuk listing.
pub const DEFAULT_LIMIT: i64 = 20;
/// Maksimum baris untuk CSV export.
pub const CSV_MAX: i64 = 10_000;

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreatePelatihanParams<'a> {
    pub poster_id: Uuid,
    pub judul: &'a str,
    pub penyelenggara: &'a str,
    pub deskripsi: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub created_by_role: &'a str,
    pub initial_status: &'a str,
    pub jumlah_peserta: Option<i32>,
}

/// Params untuk `update_pelatihan()` — grouping untuk menghindari too_many_arguments.
pub struct UpdatePelatihanParams<'a> {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: &'a str,
    pub penyelenggara: &'a str,
    pub deskripsi: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub jumlah_peserta: Option<i32>,
}

/// Params untuk `update()` (PATCH user) — semua field Option<T> untuk partial update.
pub struct PatchPelatihanParams {
    pub judul: Option<String>,
    pub penyelenggara: Option<String>,
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

#[allow(async_fn_in_trait)]
pub trait IklanPelatihanRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPelatihan>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPelatihan>, anyhow::Error>;
    async fn create(
        &self,
        params: CreatePelatihanParams<'_>,
    ) -> Result<IklanPelatihan, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    // ── PATCH (partial update by owner) ─────────────────────────────────

    async fn update(
        &self,
        id: Uuid,
        poster_id: Uuid,
        params: PatchPelatihanParams,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error>;

    // ── Admin moderation ──────────────────────────────────────────────────

    async fn admin_pelatihan_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error>;
    async fn admin_pelatihan_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<IklanPelatihan>, anyhow::Error>;
    async fn update_pelatihan(
        &self,
        params: UpdatePelatihanParams<'_>,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error>;
    async fn review_pelatihan(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<IklanPelatihan>, anyhow::Error>;
    async fn soft_delete_pelatihan(&self, id: Uuid, poster_id: Uuid)
        -> Result<bool, anyhow::Error>;

    // ── Suspension (existing) ────────────────────────────────────────────

    async fn admin_list(
        &self,
        params: AdminListParams,
    ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPelatihan>, anyhow::Error>;
    async fn suspend(
        &self,
        iklan_ids: &[Uuid],
        is_permanent: bool,
        reason: &str,
        evidence_object_key: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
    ) -> Result<Vec<IklanSuspension>, anyhow::Error>;
    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error>;
    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error>;
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;

    // ── Enrollment ───────────────────────────────────────────────────────

    async fn create_enrollment(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<PelatihanEnrollment, anyhow::Error>;
    async fn find_enrollment_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error>;
    async fn commit_enrollment_bukti(
        &self,
        id: Uuid,
        user_id: Uuid,
        object_key: &str,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error>;
    async fn admin_enrollment_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<PelatihanEnrollment>, anyhow::Error>;
    async fn admin_enrollment_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<PelatihanEnrollment>, anyhow::Error>;
    async fn review_enrollment(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<PelatihanEnrollment>, anyhow::Error>;

    // ── Badge ────────────────────────────────────────────────────────────

    async fn create_badge(
        &self,
        pelatihan_id: Uuid,
        user_id: Uuid,
    ) -> Result<PelatihanBadge, anyhow::Error>;
    async fn find_badge_by_id(&self, id: Uuid) -> Result<Option<PelatihanBadge>, anyhow::Error>;
    async fn commit_badge_sertifikat(
        &self,
        id: Uuid,
        user_id: Uuid,
        object_key: &str,
    ) -> Result<Option<PelatihanBadge>, anyhow::Error>;
    async fn admin_badge_list(
        &self,
        params: ListParams,
    ) -> Result<AdminListResult<PelatihanBadge>, anyhow::Error>;
    async fn admin_badge_list_all(
        &self,
        params: ListParams,
    ) -> Result<Vec<PelatihanBadge>, anyhow::Error>;
    async fn review_badge(
        &self,
        id: Uuid,
        approved: bool,
        review_note: Option<&str>,
        reviewer_id: Uuid,
    ) -> Result<Option<PelatihanBadge>, anyhow::Error>;
}
