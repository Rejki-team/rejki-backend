use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanPelatihanSummary {
    pub id: Uuid,
    pub judul: String,
    pub penyelenggara: String,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPelatihanClient: Send + Sync {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanPelatihanSummary, IklanPelatihanClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanPelatihanClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanPelatihanClientError {
    #[error("iklan pelatihan not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
