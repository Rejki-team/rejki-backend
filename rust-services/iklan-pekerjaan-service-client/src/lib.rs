use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanPekerjaanSummary {
    pub id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub poster_id: Uuid,
}

/// `#[async_trait]` wajib (bukan native `async fn in trait`) — trait ini kini dipakai
/// sebagai `Arc<dyn IklanPekerjaanClient>` oleh `rating-service` (F-17, Kelompok 3
/// Phase 5), native async-fn-in-trait tidak dyn-compatible. Pola sama `IklanPekerjaClient`.
#[async_trait::async_trait]
pub trait IklanPekerjaanClient: Send + Sync {
    async fn get_summary(
        &self,
        id: Uuid,
    ) -> Result<IklanPekerjaanSummary, IklanPekerjaanClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaanClientError>;

    /// Rating dua arah (F-17, Kelompok 3 Phase 5, PRD §5.15) hanya valid bila lamaran
    /// ANTARA `poster_id` (pemberi kerja) dan `pelamar_id` pada `iklan_id` ini sudah
    /// berstatus Selesai. Validasi ownership (poster_id cocok) + status dilakukan
    /// SEPENUHNYA di sisi `iklan-pekerjaan-service` (satu round-trip, tidak membocorkan
    /// field internal Lamaran/Iklan ke domain lain).
    async fn is_lamaran_selesai(
        &self,
        iklan_id: Uuid,
        poster_id: Uuid,
        pelamar_id: Uuid,
    ) -> Result<bool, IklanPekerjaanClientError>;

    /// Suspend satu iklan (P9.0c, Kelompok 6 Q9) — dipakai endpoint
    /// approve-and-suspend (`report-service`) saat `target_ad_type=pekerjaan`.
    /// Pola sama endpoint suspend mandiri existing di service ini (P9.2, tetap
    /// dipertahankan terpisah) — cross-service, jadi tanpa `evidence_object_key`
    /// (bukti sudah ada di aduan `report.report` sendiri).
    async fn suspend(
        &self,
        iklan_id: Uuid,
        is_permanent: bool,
        reason: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        admin_id: Uuid,
    ) -> Result<(), IklanPekerjaanClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanPekerjaanClientError {
    #[error("iklan pekerjaan not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
