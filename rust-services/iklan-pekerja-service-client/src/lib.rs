use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanPekerjaSummary {
    pub id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaClient: Send + Sync {
    async fn get_summary(&self, id: Uuid) -> Result<IklanPekerjaSummary, IklanPekerjaClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanPekerjaClientError {
    #[error("iklan pekerja not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
