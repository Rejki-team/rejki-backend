use super::entity::IklanPelatihan;
use uuid::Uuid;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPelatihanRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPelatihan>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPelatihan>, anyhow::Error>;
    async fn create(
        &self,
        poster_id: Uuid,
        judul: &str,
        penyelenggara: &str,
        deskripsi: &str,
    ) -> Result<IklanPelatihan, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;
}
