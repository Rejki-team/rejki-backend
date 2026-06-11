use super::dto::{CreateIklanPelatihanInput, IklanPelatihanResponse, ListQuery};
use crate::domain::repository::IklanPelatihanRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct IklanPelatihanService<R: IklanPelatihanRepository> {
    repo: Arc<R>,
}

impl<R: IklanPelatihanRepository> IklanPelatihanService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
    pub async fn list(&self, q: ListQuery) -> Result<Vec<IklanPelatihanResponse>, anyhow::Error> {
        Ok(self
            .repo
            .list(q.limit.unwrap_or(20), q.offset.unwrap_or(0))
            .await?
            .into_iter()
            .map(to_resp)
            .collect())
    }
    pub async fn get(&self, id: Uuid) -> Result<IklanPelatihanResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_resp)
            .ok_or_else(|| anyhow::anyhow!("tidak ditemukan"))
    }
    pub async fn create(
        &self,
        poster_id: Uuid,
        input: CreateIklanPelatihanInput,
    ) -> Result<IklanPelatihanResponse, anyhow::Error> {
        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        Ok(to_resp(
            self.repo
                .create(poster_id, &judul, &input.penyelenggara, &deskripsi)
                .await?,
        ))
    }
    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }
}

fn to_resp(e: crate::domain::entity::IklanPelatihan) -> IklanPelatihanResponse {
    IklanPelatihanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        penyelenggara: e.penyelenggara,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        harga: e.harga,
        tanggal_mulai: e.tanggal_mulai,
        tanggal_selesai: e.tanggal_selesai,
        is_active: e.is_active,
        created_at: e.created_at,
    }
}
