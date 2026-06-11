use super::entity::IklanPekerja;
use uuid::Uuid;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerja>, anyhow::Error>;
    async fn create(
        &self,
        poster_id: Uuid,
        nama: &str,
        keahlian: &[String],
        deskripsi: &str,
    ) -> Result<IklanPekerja, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;
}
