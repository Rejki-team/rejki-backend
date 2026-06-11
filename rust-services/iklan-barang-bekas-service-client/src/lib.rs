use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanBarangBekasSummary {
    pub id: Uuid,
    pub judul: String,
    pub harga: i64,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanBarangBekasClient: Send + Sync {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanBarangBekasSummary, IklanBarangBekasClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanBarangBekasClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanBarangBekasClientError {
    #[error("iklan barang bekas not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
