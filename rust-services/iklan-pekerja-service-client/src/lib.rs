use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IklanPekerjaSummary {
    pub id: Uuid,
    /// Pemilik iklan pekerja ini — dipakai untuk korelasi balik saat batch lookup
    /// (mis. `get_active_summaries_for_posters`), karena panggilan itu di-index by
    /// `poster_id`, bukan `id` iklan.
    pub poster_id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
    /// Foto profil pertama (PRD §5.11.5 "foto profil" di kartu Kelola Pelamar) —
    /// `None` bila iklan pekerja tidak punya foto.
    pub foto_url: Option<String>,
}

// `dyn`-compatible (dipakai sebagai `Arc<dyn IklanPekerjaClient>` yang di-inject ke
// service lain, sama seperti RegionClient/GeocodingClient) — perlu async-trait, bukan
// native `async fn in trait` (yang tidak object-safe).
#[async_trait::async_trait]
pub trait IklanPekerjaClient: Send + Sync {
    async fn get_summary(&self, id: Uuid) -> Result<IklanPekerjaSummary, IklanPekerjaClientError>;
    async fn exists(&self, id: Uuid) -> Result<bool, IklanPekerjaClientError>;
    /// P1.3 validasi #1 (F-3): apakah `poster_id` sudah punya Iklan Pekerja aktif
    /// (prasyarat wajib sebelum bisa melamar pekerjaan orang lain).
    async fn exists_active_for_poster(
        &self,
        poster_id: Uuid,
    ) -> Result<bool, IklanPekerjaClientError>;

    /// P1.10 enrichment (F-3/F-4, Kelompok 3 Phase 2): ambil ringkasan Iklan Pekerja
    /// AKTIF untuk sekumpulan `poster_id` sekaligus (satu query batched, BUKAN loop
    /// per-poster — Hazard #5). Dipakai `iklan-pekerjaan-service` untuk melengkapi
    /// kartu "Kelola Pelamar" (nama, kode Iklan Pekerja) tanpa N+1. Poster tanpa
    /// Iklan Pekerja aktif tidak muncul di hasil (bukan error).
    async fn get_active_summaries_for_posters(
        &self,
        poster_ids: &[Uuid],
    ) -> Result<Vec<IklanPekerjaSummary>, IklanPekerjaClientError>;

    /// Suspend satu iklan (P9.0c, Kelompok 6 Q9) — dipakai endpoint
    /// approve-and-suspend (`report-service`) saat `target_ad_type=pekerja`.
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
    ) -> Result<(), IklanPekerjaClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum IklanPekerjaClientError {
    #[error("iklan pekerja not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
