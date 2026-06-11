use super::dto::{CreateIklanPekerjaInput, IklanPekerjaResponse, ListQuery};
use crate::domain::repository::IklanPekerjaRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct IklanPekerjaService<R: IklanPekerjaRepository> {
    repo: Arc<R>,
}

impl<R: IklanPekerjaRepository> IklanPekerjaService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanPekerjaResponse>, anyhow::Error> {
        Ok(self
            .repo
            .list(q.limit.unwrap_or(20), q.offset.unwrap_or(0))
            .await?
            .into_iter()
            .map(to_response)
            .collect())
    }
    pub async fn get(&self, id: Uuid) -> Result<IklanPekerjaResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_response)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }
    pub async fn create(
        &self,
        poster_id: Uuid,
        input: CreateIklanPekerjaInput,
    ) -> Result<IklanPekerjaResponse, anyhow::Error> {
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        Ok(to_response(
            self.repo
                .create(poster_id, &input.nama, &input.keahlian, &deskripsi)
                .await?,
        ))
    }
    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }
}

fn to_response(e: crate::domain::entity::IklanPekerja) -> IklanPekerjaResponse {
    IklanPekerjaResponse {
        id: e.id,
        poster_id: e.poster_id,
        nama: e.nama,
        keahlian: e.keahlian,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        tarif_min: e.tarif_min,
        tarif_max: e.tarif_max,
        is_active: e.is_active,
        created_at: e.created_at,
    }
}
