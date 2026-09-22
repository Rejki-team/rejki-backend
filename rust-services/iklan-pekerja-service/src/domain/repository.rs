use super::entity::{IklanPekerja, IklanSuspension};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AdminListParams {
    pub q: Option<String>,
    pub moderation_status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct AdminListResult {
    pub items: Vec<IklanPekerja>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct UpdatePekerjaParams {
    pub nama: Option<String>,
    pub keahlian: Option<Vec<String>>,
    pub deskripsi: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub jam_kerja: Option<String>,
    pub phone_number: Option<String>,
    pub foto_urls: Option<Vec<String>>,
    pub is_active: Option<bool>,
    /// Hasil re-geocoding (F-1) bila `lokasi`/`region_id` berubah. `None` = tidak berubah
    /// ATAU geocoding gagal — kolom lama dipertahankan (COALESCE), degradasi anggun.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreatePekerjaParams<'a> {
    pub poster_id: Uuid,
    pub nama: &'a str,
    pub keahlian: &'a [String],
    pub deskripsi: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub jam_kerja: Option<&'a str>,
    pub phone_number: Option<&'a str>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[allow(async_fn_in_trait)]
pub trait IklanPekerjaRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error>;
    /// `radius`: filter "dalam radius X km dari koordinat pengguna" (F-1, PRD §5.12.1) —
    /// `None` = tidak difilter (semua iklan aktif, seperti sebelumnya).
    async fn list(
        &self,
        limit: i64,
        offset: i64,
        radius: Option<common_geo::RadiusQuery>,
    ) -> Result<Vec<IklanPekerja>, anyhow::Error>;
    async fn create(&self, params: CreatePekerjaParams<'_>) -> Result<IklanPekerja, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPekerja>, anyhow::Error>;
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
    async fn update(
        &self,
        id: Uuid,
        poster_id: Uuid,
        params: UpdatePekerjaParams,
    ) -> Result<Option<IklanPekerja>, anyhow::Error>;
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;

    /// Dipakai `IklanPekerjaClient::exists_active_for_poster` (F-3, Kelompok 3 Phase 1) —
    /// prasyarat "sudah punya Iklan Pekerja" sebelum bisa melamar pekerjaan orang lain.
    async fn exists_active_for_poster(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;

    /// Dipakai `IklanPekerjaClient::get_active_summaries_for_posters` (F-3/F-4,
    /// Kelompok 3 Phase 2) — batch lookup, satu query untuk banyak `poster_id`.
    async fn find_active_by_posters(
        &self,
        poster_ids: &[Uuid],
    ) -> Result<Vec<IklanPekerja>, anyhow::Error>;
}
