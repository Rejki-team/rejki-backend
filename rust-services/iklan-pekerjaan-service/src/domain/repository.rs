use super::entity::IklanPekerjaan;
use uuid::Uuid;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaanRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerjaan>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    async fn create(
        &self,
        poster_id: Uuid,
        judul: &str,
        perusahaan: &str,
        deskripsi: &str,
        tipe: &str,
    ) -> Result<IklanPekerjaan, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;
}
