use super::dto::{CreateIklanBarangBekasInput, IklanBarangBekasResponse, ListQuery};
use crate::domain::repository::IklanBarangBekasRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct IklanBarangBekasService<R: IklanBarangBekasRepository> {
    repo: Arc<R>,
}

impl<R: IklanBarangBekasRepository> IklanBarangBekasService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanBarangBekasResponse>, anyhow::Error> {
        Ok(self
            .repo
            .list(q.limit.unwrap_or(20), q.offset.unwrap_or(0))
            .await?
            .into_iter()
            .map(to_resp)
            .collect())
    }
    pub async fn get(&self, id: Uuid) -> Result<IklanBarangBekasResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }
    pub async fn create(
        &self,
        seller_id: Uuid,
        input: CreateIklanBarangBekasInput,
    ) -> Result<IklanBarangBekasResponse, anyhow::Error> {
        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        Ok(to_resp(
            self.repo
                .create(seller_id, &judul, &deskripsi, input.harga, &input.kondisi)
                .await?,
        ))
    }
    pub async fn mark_sold(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.mark_sold(id, seller_id).await
    }
    pub async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, seller_id).await
    }
}

fn to_resp(e: crate::domain::entity::IklanBarangBekas) -> IklanBarangBekasResponse {
    IklanBarangBekasResponse {
        id: e.id,
        seller_id: e.seller_id,
        judul: e.judul,
        deskripsi: e.deskripsi,
        harga: e.harga,
        kondisi: e.kondisi,
        lokasi: e.lokasi,
        is_sold: e.is_sold,
        created_at: e.created_at,
    }
}
