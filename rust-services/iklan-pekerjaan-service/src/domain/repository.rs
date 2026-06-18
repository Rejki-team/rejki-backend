use super::entity::{IklanPekerjaan, IklanSuspension};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Parameter untuk listing admin: search, filter, sort, pagination.
#[derive(Debug, Clone)]
pub struct AdminListParams {
    pub q: Option<String>,
    pub moderation_status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Hasil listing admin dengan total count.
#[derive(Debug, Clone)]
pub struct AdminListResult {
    pub items: Vec<IklanPekerjaan>,
    pub total: i64,
}

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreatePekerjaanParams<'a> {
    pub poster_id: Uuid,
    pub judul: &'a str,
    pub perusahaan: &'a str,
    pub deskripsi: &'a str,
    pub tipe: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaanRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerjaan>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    async fn create(
        &self,
        params: CreatePekerjaanParams<'_>,
    ) -> Result<IklanPekerjaan, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    // ── Admin endpoints ──
    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
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

    /// Kembalikan iklan `suspended_temp` yang `expires_at <= now()` ke `active`.
    /// Mengembalikan jumlah iklan yang di-un-suspend.
    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error>;

    /// Cek apakah poster sedang dalam cooldown 3 hari akibat suspend permanen.
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;
}
