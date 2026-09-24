use std::sync::Arc;

use uuid::Uuid;

use iklan_pekerja_service_client::{
    IklanPekerjaClient, IklanPekerjaClientError, IklanPekerjaSummary,
};

use crate::domain::repository::IklanPekerjaRepository;
use crate::infrastructure::pg_repository::PgIklanPekerjaRepository;

/// Implementasi `IklanPekerjaClient` untuk mode in-process (Modular Monolith).
/// Dipakai `rejki-app` sebagai `Arc<dyn IklanPekerjaClient>` yang di-inject ke
/// `iklan-pekerjaan-service` (F-3, validasi P1.3 "sudah punya Iklan Pekerja").
///
/// Memegang tipe KONKRET `PgIklanPekerjaRepository` (bukan generik `R: IklanPekerjaRepository`)
/// — native `async fn in trait` tidak menjamin `Send` di konteks generik, sehingga
/// `#[async_trait]` (dyn-compatible, dipakai trait `IklanPekerjaClient`) gagal kompilasi
/// bila field-nya generik. Pola sama dengan `common-scheduler::JobHandler` di
/// `iklan-pelatihan-service`. Composition Root selalu memakai Postgres.
pub struct IklanPekerjaInProcessClient {
    repo: Arc<PgIklanPekerjaRepository>,
}

impl IklanPekerjaInProcessClient {
    pub fn new(repo: Arc<PgIklanPekerjaRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl IklanPekerjaClient for IklanPekerjaInProcessClient {
    async fn get_summary(&self, id: Uuid) -> Result<IklanPekerjaSummary, IklanPekerjaClientError> {
        let iklan = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| IklanPekerjaClientError::Unavailable)?
            .ok_or(IklanPekerjaClientError::NotFound)?;
        Ok(IklanPekerjaSummary {
            id: iklan.id,
            poster_id: iklan.poster_id,
            nama: iklan.nama,
            keahlian: iklan.keahlian,
            foto_url: iklan.foto_urls.first().cloned(),
        })
    }

    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaClientError> {
        self.repo
            .exists(id)
            .await
            .map_err(|_| IklanPekerjaClientError::Unavailable)
    }

    async fn exists_active_for_poster(
        &self,
        poster_id: Uuid,
    ) -> Result<bool, IklanPekerjaClientError> {
        self.repo
            .exists_active_for_poster(poster_id)
            .await
            .map_err(|_| IklanPekerjaClientError::Unavailable)
    }

    async fn get_active_summaries_for_posters(
        &self,
        poster_ids: &[Uuid],
    ) -> Result<Vec<IklanPekerjaSummary>, IklanPekerjaClientError> {
        let items = self
            .repo
            .find_active_by_posters(poster_ids)
            .await
            .map_err(|_| IklanPekerjaClientError::Unavailable)?;
        Ok(items
            .into_iter()
            .map(|i| IklanPekerjaSummary {
                id: i.id,
                poster_id: i.poster_id,
                nama: i.nama,
                keahlian: i.keahlian,
                foto_url: i.foto_urls.first().cloned(),
            })
            .collect())
    }

    async fn suspend(
        &self,
        iklan_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        admin_id: Uuid,
    ) -> Result<(), IklanPekerjaClientError> {
        let suspended = self
            .repo
            .suspend(
                &[iklan_id],
                is_permanent,
                reason,
                None,
                expires_at,
                admin_id,
            )
            .await
            .map_err(|_| IklanPekerjaClientError::Unavailable)?;
        if suspended.is_empty() {
            return Err(IklanPekerjaClientError::NotFound);
        }
        Ok(())
    }
}
