use std::sync::Arc;

use uuid::Uuid;

use iklan_barang_bekas_service_client::{
    IklanBarangBekasClient, IklanBarangBekasClientError, IklanBarangBekasSummary,
};

use crate::domain::repository::IklanBarangBekasRepository;
use crate::infrastructure::pg_repository::PgIklanBarangBekasRepository;

/// Implementasi `IklanBarangBekasClient` untuk mode in-process (Modular
/// Monolith) — P9.0c (Kelompok 6 Q9). Dipakai `rejki-app` sebagai `Arc<dyn
/// IklanBarangBekasClient>` yang di-inject ke `report-service` untuk endpoint
/// approve-and-suspend. Sebelum task ini trait tidak punya implementor sama
/// sekali (belum ada consumer lintas-service) — file ini baru.
///
/// Memegang tipe KONKRET `PgIklanBarangBekasRepository` — pola sama
/// `IklanPekerjaInProcessClient`/`IklanPekerjaanInProcessClient` (native
/// `async fn in trait` generik tidak menjamin `Send` untuk `#[async_trait]`
/// dyn-compatible).
pub struct IklanBarangBekasInProcessClient {
    repo: Arc<PgIklanBarangBekasRepository>,
}

impl IklanBarangBekasInProcessClient {
    pub fn new(repo: Arc<PgIklanBarangBekasRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl IklanBarangBekasClient for IklanBarangBekasInProcessClient {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanBarangBekasSummary, IklanBarangBekasClientError> {
        let iklan = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|_| IklanBarangBekasClientError::Unavailable)?
            .ok_or(IklanBarangBekasClientError::NotFound)?;
        Ok(IklanBarangBekasSummary {
            id: iklan.id,
            judul: iklan.judul,
            // Entity `IklanBarangBekas` TIDAK punya field harga (barang gratis,
            // lihat komentar domain — fokus jenis/jumlah/lokasi pengambilan).
            // `IklanBarangBekasSummary.harga` sudah ada di kontrak client sejak
            // sebelum task ini (trait tanpa implementor) — dipertahankan
            // sebagai 0 (bukan scope task P9.0c untuk mengubah kontrak DTO ini).
            harga: 0,
        })
    }

    async fn exists(&self, id: Uuid) -> Result<bool, IklanBarangBekasClientError> {
        self.repo
            .exists(id)
            .await
            .map_err(|_| IklanBarangBekasClientError::Unavailable)
    }

    async fn suspend(
        &self,
        iklan_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        admin_id: Uuid,
    ) -> Result<(), IklanBarangBekasClientError> {
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
            .map_err(|_| IklanBarangBekasClientError::Unavailable)?;
        if suspended.is_empty() {
            return Err(IklanBarangBekasClientError::NotFound);
        }
        Ok(())
    }
}
