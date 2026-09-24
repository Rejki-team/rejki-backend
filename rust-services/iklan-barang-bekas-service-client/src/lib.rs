use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanBarangBekasSummary {
    pub id: Uuid,
    pub judul: String,
    pub harga: i64,
}

// `dyn`-compatible (P9.0c, Kelompok 6 Q9 — dipakai sebagai `Arc<dyn
// IklanBarangBekasClient>` oleh `report-service` untuk endpoint approve-and-suspend,
// pola sama `IklanPekerjaanClient`/`IklanPekerjaClient`) — perlu async-trait, bukan
// native `async fn in trait` (yang tidak object-safe). Diubah dari
// `#[allow(async_fn_in_trait)]` karena sebelumnya trait ini tidak pernah dipakai
// lintas-service (no implementor/consumer) — sekarang jadi kebutuhan nyata.
#[async_trait::async_trait]
pub trait IklanBarangBekasClient: Send + Sync {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanBarangBekasSummary, IklanBarangBekasClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanBarangBekasClientError>;

    /// Suspend satu iklan (P9.0c) — dipakai endpoint approve-and-suspend
    /// (`report-service`) saat `target_ad_type=barang_bekas`. Pola sama
    /// endpoint suspend mandiri existing di service ini (P9.2, tetap
    /// dipertahankan terpisah) — cross-service, jadi tanpa
    /// `evidence_object_key` (bukti sudah ada di aduan `report.report` sendiri).
    async fn suspend(
        &self,
        iklan_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        admin_id: Uuid,
    ) -> Result<(), IklanBarangBekasClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanBarangBekasClientError {
    #[error("iklan barang bekas not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
