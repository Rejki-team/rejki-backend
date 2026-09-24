use std::sync::Arc;

use uuid::Uuid;

use iklan_pekerjaan_service_client::{
    IklanPekerjaanClient, IklanPekerjaanClientError, IklanPekerjaanSummary,
};

use crate::domain::entity::LamaranStatus;
use crate::domain::repository::IklanPekerjaanRepository;
use crate::infrastructure::pg_repository::PgIklanPekerjaanRepository;

/// Implementasi `IklanPekerjaanClient` untuk mode in-process (Modular Monolith).
/// Dipakai `rating-service` untuk validasi rating dua arah (F-17, Kelompok 3
/// Phase 5, PRD §5.15): "penilaian hanya terbuka setelah aktifitas Selesai".
///
/// Memegang tipe KONKRET `PgIklanPekerjaanRepository` — pola sama
/// `IklanPekerjaInProcessClient` (native `async fn in trait` generik tidak
/// menjamin `Send` untuk `#[async_trait]` dyn-compatible).
pub struct IklanPekerjaanInProcessClient {
    repo: Arc<PgIklanPekerjaanRepository>,
}

impl IklanPekerjaanInProcessClient {
    pub fn new(repo: Arc<PgIklanPekerjaanRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl IklanPekerjaanClient for IklanPekerjaanInProcessClient {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanPekerjaanSummary, IklanPekerjaanClientError> {
        let iklan = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| IklanPekerjaanClientError::Unavailable)?
            .ok_or(IklanPekerjaanClientError::NotFound)?;
        Ok(IklanPekerjaanSummary {
            id: iklan.id,
            judul: iklan.judul,
            perusahaan: iklan.perusahaan,
            poster_id: iklan.poster_id,
        })
    }

    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaanClientError> {
        self.repo
            .exists(id)
            .await
            .map_err(|_| IklanPekerjaanClientError::Unavailable)
    }

    async fn is_lamaran_selesai(
        &self,
        iklan_id: Uuid,
        poster_id: Uuid,
        pelamar_id: Uuid,
    ) -> Result<bool, IklanPekerjaanClientError> {
        let iklan = self
            .repo
            .find_by_id(iklan_id)
            .await
            .map_err(|_| IklanPekerjaanClientError::Unavailable)?
            .ok_or(IklanPekerjaanClientError::NotFound)?;
        if iklan.poster_id != poster_id {
            return Ok(false);
        }
        let lamaran = self
            .repo
            .find_lamaran_by_iklan_and_pelamar(iklan_id, pelamar_id)
            .await
            .map_err(|_| IklanPekerjaanClientError::Unavailable)?;
        Ok(lamaran.is_some_and(|l| l.status == LamaranStatus::Selesai))
    }

    async fn suspend(
        &self,
        iklan_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        admin_id: Uuid,
    ) -> Result<(), IklanPekerjaanClientError> {
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
            .map_err(|_| IklanPekerjaanClientError::Unavailable)?;
        if suspended.is_empty() {
            return Err(IklanPekerjaanClientError::NotFound);
        }
        Ok(())
    }
}
