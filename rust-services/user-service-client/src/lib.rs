use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
}

/// Batch lookup (Hazard #5, Kelompok 3 Phase 3, F-15 "daftar bider") — nama + region id
/// mentah (village/district, BUKAN nama — pemanggil resolve nama via `RegionClient` sendiri,
/// konsisten dgn pola lain) + koordinat mentah HANYA untuk dipakai IN-PROCESS oleh pemanggil
/// menghitung jarak sendiri via `common_geo::haversine_km`. Koordinat/region id di sini TIDAK
/// PERNAH boleh diserialisasi balik ke response HTTP publik pemanggil — lihat
/// `IklanBarangBekasService::list_bider` (barang-bekas-service) yang mengonsumsi ini dan hanya
/// mengekspos `kelurahan`/`kecamatan` (nama) + `jarak_km` ke client.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserLocationSummary {
    /// `auth_id` (bukan `user_svc.profiles.id`) — konsisten dengan seluruh domain lain
    /// (poster_id/seller_id/pelamar_id/peminat_id di database service lain memakai id ini).
    pub auth_id: Uuid,
    pub username: String,
    pub village_id: Option<String>,
    pub district_id: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Data demografis pelapor untuk halaman admin Pengelolaan Dukungan (F-21/F-22,
/// PRD §6.10 — "Tingkat Pendidikan, Jenis Kelamin, Tempat Tanggal Lahir, Alamat
/// Domisili, Kelurahan+Kecamatan+Kota/Kabupaten+Provinsi+Negara"). Region id MENTAH
/// (BUKAN nama) — pemanggil resolve nama via `RegionClient` sendiri, pola sama
/// `UserLocationSummary` (Kelompok 3 F-15).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserDemographicSummary {
    pub auth_id: Uuid,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub address_line: Option<String>,
    pub village_id: Option<String>,
    pub district_id: Option<String>,
    pub regency_id: Option<String>,
    pub province_id: Option<String>,
    pub country_code: String,
}

/// Indikator ketersediaan dokumen sensitif (NIK/KTP/Selfie) milik seorang user —
/// TANPA membuka isinya, TANPA audit (bukan "read", murni existence check untuk
/// UI, mis. badge di halaman admin lain). Lihat `UserClient::get_sensitive_doc_flags`.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct SensitiveDocFlags {
    pub has_nik: bool,
    pub has_ktp: bool,
    pub has_selfie: bool,
}

/// Trait client untuk user-service — dipakai in-process & via HTTP client.
/// `#[async_trait]` diperlukan agar dyn-compatible (auth-service me-wrap sebagai
/// `Arc<dyn UserClient>`).
#[async_trait::async_trait]
pub trait UserClient: Send + Sync {
    async fn get_user_summary(&self, user_id: Uuid) -> Result<UserSummary, UserClientError>;
    async fn user_exists(&self, user_id: Uuid) -> Result<bool, UserClientError>;
    /// Musnahkan dokumen KYC milik pengguna (KTP + Selfie) dari storage +
    /// kosongkan referensi. Dipicu saat suspend permanen (extend-user-suspension-bulk-purge D4).
    /// Idempoten: aman dipanggil ulang.
    async fn purge_kyc_documents(&self, user_id: Uuid) -> Result<(), UserClientError>;

    /// Indikator ketersediaan NIK/KTP/Selfie milik `auth_id` — dipakai halaman admin
    /// lain (mis. Iklan Pekerja) untuk menampilkan badge "dokumen tersedia" pada
    /// detail, TANPA menyalin data sensitif ke schema pemanggil (F-27b).
    async fn get_sensitive_doc_flags(
        &self,
        auth_id: Uuid,
    ) -> Result<SensitiveDocFlags, UserClientError>;

    /// Proxy reveal NIK penuh untuk admin dari service lain (F-27b). Audit
    /// `nik_read_issued` TETAP tercatat tunggal di user-service — pemanggil TIDAK
    /// boleh menyimpan/mencatat ulang. `Ok(None)` bila NIK belum ada (bukan error).
    async fn admin_reveal_nik(
        &self,
        auth_id: Uuid,
        admin_id: Uuid,
    ) -> Result<Option<String>, UserClientError>;

    /// Proxy presigned read URL dokumen (`kind`: "ktp" | "selfie") untuk admin dari
    /// service lain (F-27b). Audit `read_issued` tetap tunggal di user-service.
    /// `Ok(None)` bila dokumen belum ada (bukan error).
    async fn admin_get_document_url(
        &self,
        auth_id: Uuid,
        kind: &str,
        admin_id: Uuid,
    ) -> Result<Option<String>, UserClientError>;

    /// Batch lookup nama + lokasi mentah untuk beberapa `auth_id` sekaligus (Hazard #5) —
    /// dipakai "daftar bider" (F-15). Entri untuk `auth_id` yang tidak ditemukan/tidak punya
    /// profil TIDAK disertakan (bukan error) — pemanggil menangani degradasi anggun.
    async fn get_location_summaries_by_auth_ids(
        &self,
        auth_ids: &[Uuid],
    ) -> Result<Vec<UserLocationSummary>, UserClientError>;

    /// Batch lookup data demografis untuk beberapa `auth_id` sekaligus (Hazard #5) —
    /// dipakai halaman admin Pengelolaan Dukungan (F-21/F-22). Entri untuk `auth_id`
    /// yang tidak ditemukan/tidak punya profil TIDAK disertakan (bukan error).
    async fn get_demographic_summaries_by_auth_ids(
        &self,
        auth_ids: &[Uuid],
    ) -> Result<Vec<UserDemographicSummary>, UserClientError>;

    /// Batch lookup `UserSummary` (username+avatar) untuk beberapa `auth_id` sekaligus
    /// (Hazard #5) — dipakai "daftar percakapan" chat-service (F-18, Kelompok 4 Phase 5,
    /// P4.10) untuk mengisi nama+foto profil lawan bicara tanpa N+1. Entri untuk
    /// `auth_id` yang tidak ditemukan/tidak punya profil TIDAK disertakan (bukan error).
    async fn get_summaries_by_auth_ids(
        &self,
        auth_ids: &[Uuid],
    ) -> Result<Vec<UserSummary>, UserClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum UserClientError {
    #[error("user not found")]
    NotFound,
    #[error("user service unavailable")]
    Unavailable,
}
