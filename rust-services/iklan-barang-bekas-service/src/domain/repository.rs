use super::entity::IklanBarangBekas;
use uuid::Uuid;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanBarangBekasRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
    async fn create(
        &self,
        seller_id: Uuid,
        judul: &str,
        deskripsi: &str,
        harga: i64,
        kondisi: &str,
    ) -> Result<IklanBarangBekas, anyhow::Error>;
    async fn mark_sold(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;
}
