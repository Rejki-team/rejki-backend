use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanPekerjaanSummary {
    pub id: Uuid,
    pub judul: String,
    pub perusahaan: String,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaanClient: Send + Sync {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanPekerjaanSummary, IklanPekerjaanClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaanClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanPekerjaanClientError {
    #[error("iklan pekerjaan not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
