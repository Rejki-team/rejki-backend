use super::entity::{IklanBarangBekas, IklanSuspension};
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
    pub items: Vec<IklanBarangBekas>,
    pub total: i64,
}

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreateBarangBekasParams<'a> {
    pub seller_id: Uuid,
    pub judul: &'a str,
    pub deskripsi: &'a str,
    pub jenis_barang: &'a str,
    pub jumlah: i32,
    pub lokasi_pengambilan: &'a str,
    pub lokasi: Option<&'a str>,
    pub foto_urls: &'a [String],
}

#[allow(async_fn_in_trait)]
pub trait IklanBarangBekasRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
    async fn create(
        &self,
        params: CreateBarangBekasParams<'_>,
    ) -> Result<IklanBarangBekas, anyhow::Error>;
    /// Tandai barang sebagai "sudah diambil". Hanya pemilik (seller_id).
    /// Kembalikan true bila berhasil, false bila bukan pemilik/iklan tidak ditemukan.
    async fn mark_taken(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
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
