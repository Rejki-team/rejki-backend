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

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreatePekerjaParams<'a> {
    pub poster_id: Uuid,
    pub nama: &'a str,
    pub keahlian: &'a [String],
    pub deskripsi: &'a str,
    pub lokasi: Option<&'a str>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
}

#[allow(async_fn_in_trait)]
pub trait IklanPekerjaRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerja>, anyhow::Error>;
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
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;
}
