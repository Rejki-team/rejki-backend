use std::sync::Arc;
use uuid::Uuid;

use super::dto::{CreateIklanPekerjaanInput, IklanPekerjaanResponse, ListQuery};
use crate::domain::repository::IklanPekerjaanRepository;

pub struct IklanPekerjaanService<R: IklanPekerjaanRepository> {
    repo: Arc<R>,
}

impl<R: IklanPekerjaanRepository> IklanPekerjaanService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        query: ListQuery,
    ) -> Result<Vec<IklanPekerjaanResponse>, anyhow::Error> {
        let items = self
            .repo
            .list(query.limit.unwrap_or(20), query.offset.unwrap_or(0))
            .await?;
        Ok(items.into_iter().map(to_response).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<IklanPekerjaanResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_response)
            .ok_or_else(|| anyhow::anyhow!("iklan tidak ditemukan"))
    }

    pub async fn create(
        &self,
        poster_id: Uuid,
        input: CreateIklanPekerjaanInput,
    ) -> Result<IklanPekerjaanResponse, anyhow::Error> {
        let judul = ammonia::clean_text(&input.judul);
        let deskripsi = ammonia::clean_text(&input.deskripsi);
        let item = self
            .repo
            .create(
                poster_id,
                &judul,
                &input.perusahaan,
                &deskripsi,
                &input.tipe,
            )
            .await?;
        Ok(to_response(item))
    }

    pub async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.delete(id, poster_id).await
    }
}

fn to_response(e: crate::domain::entity::IklanPekerjaan) -> IklanPekerjaanResponse {
    IklanPekerjaanResponse {
        id: e.id,
        poster_id: e.poster_id,
        judul: e.judul,
        perusahaan: e.perusahaan,
        deskripsi: e.deskripsi,
        lokasi: e.lokasi,
        gaji_min: e.gaji_min,
        gaji_max: e.gaji_max,
        tipe: e.tipe,
        is_active: e.is_active,
        created_at: e.created_at,
    }
}
