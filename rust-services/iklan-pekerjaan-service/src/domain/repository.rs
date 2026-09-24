use super::entity::{IklanPekerjaan, IklanSuspension, Lamaran};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use uuid::Uuid;

/// Params untuk `create_lamaran()` — grouping untuk menghindari too_many_arguments.
pub struct CreateLamaranParams {
    pub iklan_id: Uuid,
    pub pelamar_id: Uuid,
    pub tanggal: NaiveDate,
    pub jam_mulai: NaiveTime,
    pub jam_akhir: NaiveTime,
    pub kuota_diambil: i32,
}

/// Parameter untuk listing admin: search, filter, sort, pagination.
#[derive(Debug, Clone)]
pub struct AdminListParams {
    pub q: Option<String>,
    pub moderation_status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Hasil listing admin dengan total count.
#[derive(Debug, Clone)]
pub struct AdminListResult {
    pub items: Vec<IklanPekerjaan>,
    pub total: i64,
}

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreatePekerjaanParams<'a> {
    pub poster_id: Uuid,
    pub judul: &'a str,
    pub perusahaan: &'a str,
    pub deskripsi: &'a str,
    pub tipe: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub jam_kerja: Option<&'a str>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Params untuk `update()` — grouping untuk menghindari too_many_arguments.
pub struct UpdatePekerjaanParams<'a> {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: Option<&'a str>,
    pub perusahaan: Option<&'a str>,
    pub deskripsi: Option<&'a str>,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: Option<&'a str>,
    pub jam_kerja: Option<&'a str>,
    pub foto_urls: Option<&'a [String]>,
    pub is_active: Option<bool>,
    /// Hasil re-geocoding (F-1) bila `lokasi`/`region_id` berubah. `None` = tidak berubah
    /// ATAU geocoding gagal — kolom lama dipertahankan (COALESCE), degradasi anggun.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait IklanPekerjaanRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerjaan>, anyhow::Error>;
    /// Batch lookup (Hazard #5, Kelompok 3 Phase 2) — dipakai `list_lamaran_for_pelamar`
    /// untuk melengkapi tiap baris dengan judul/upah/tipe/alamat iklan, satu query
    /// untuk seluruh `iklan_id` di daftar, bukan N+1 per lamaran.
    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    /// "Iklan Saya" (Kelompok 3 Phase 2) — daftar iklan milik satu poster, entry
    /// point ke "Kelola Pelamar" (PRD §5.11.5). Tidak difilter moderasi/aktif —
    /// pemilik tetap boleh lihat iklan sendiri yang di-suspend.
    async fn list_by_poster(
        &self,
        poster_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    /// `radius`: filter "dalam radius X km dari koordinat pengguna" (F-1, PRD §5.11.1) —
    /// `None` = tidak difilter (semua iklan aktif, seperti sebelumnya).
    async fn list(
        &self,
        limit: i64,
        offset: i64,
        radius: Option<common_geo::RadiusQuery>,
    ) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    async fn create(
        &self,
        params: CreatePekerjaanParams<'_>,
    ) -> Result<IklanPekerjaan, anyhow::Error>;
    async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn update(
        &self,
        params: UpdatePekerjaanParams<'_>,
    ) -> Result<IklanPekerjaan, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    // ── Admin endpoints ──
    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanPekerjaan>, anyhow::Error>;
    async fn suspend(
        &self,
        iklan_ids: &[Uuid],
        is_permanent: bool,
        reason: &str,
        evidence_object_key: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        created_by: Uuid,
    ) -> Result<Vec<IklanSuspension>, anyhow::Error>;
    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    /// Kembalikan iklan `suspended_temp` yang `expires_at <= now()` ke `active`.
    /// Mengembalikan jumlah iklan yang di-un-suspend.
    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error>;

    /// Cek apakah poster sedang dalam cooldown 3 hari akibat suspend permanen.
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;

    // ── Lamaran (F-3, Kelompok 3 Phase 1) ───────────────────────────────

    async fn find_lamaran_by_id(&self, id: Uuid) -> Result<Option<Lamaran>, anyhow::Error>;
    /// Kelompok 3 Phase 5 (F-17) — dipakai `IklanPekerjaanInProcessClient::is_lamaran_selesai`
    /// untuk validasi rating: satu pasangan (iklan_id, pelamar_id) hanya punya 1 lamaran aktif.
    async fn find_lamaran_by_iklan_and_pelamar(
        &self,
        iklan_id: Uuid,
        pelamar_id: Uuid,
    ) -> Result<Option<Lamaran>, anyhow::Error>;
    async fn create_lamaran(&self, params: CreateLamaranParams) -> Result<Lamaran, anyhow::Error>;
    /// P1.10: daftar lamaran untuk satu iklan (dipanggil pemilik iklan — "Kelola Pelamar").
    async fn list_lamaran_for_iklan(&self, iklan_id: Uuid) -> Result<Vec<Lamaran>, anyhow::Error>;
    /// P1.10: daftar lamaran milik satu pelamar (dipanggil pelamar — "Riwayat Aktifitas Pelamar").
    async fn list_lamaran_for_pelamar(
        &self,
        pelamar_id: Uuid,
    ) -> Result<Vec<Lamaran>, anyhow::Error>;
    /// P1.3 validasi #3: apakah `pelamar_id` sudah punya lamaran AKTIF (diterima/proses)
    /// yang jadwalnya (tanggal + rentang jam) bertumpang-tindih dengan yang diajukan.
    async fn has_conflicting_lamaran(
        &self,
        pelamar_id: Uuid,
        tanggal: NaiveDate,
        jam_mulai: NaiveTime,
        jam_akhir: NaiveTime,
    ) -> Result<bool, anyhow::Error>;
    /// P1.4: terima/tolak lamaran — ownership check DI QUERY (`iklan_owner_id`, IDOR→404).
    /// Hanya lamaran berstatus `Diajukan` yang bisa direview.
    async fn review_lamaran(
        &self,
        id: Uuid,
        iklan_owner_id: Uuid,
        approved: bool,
    ) -> Result<Option<Lamaran>, anyhow::Error>;
    /// P1.5: transaksi atomik (Hazard #4) — Lamaran `Diterima`→`Proses` DAN Iklan
    /// `Tersedia`→`SedangDikerjakan`. Ownership check DI QUERY (`pelamar_id`, IDOR→404).
    async fn mulai_bekerja(
        &self,
        lamaran_id: Uuid,
        pelamar_id: Uuid,
    ) -> Result<Option<Lamaran>, anyhow::Error>;
    /// P1.6: transaksi atomik — Lamaran `Proses`→`Selesai` DAN Iklan
    /// `SedangDikerjakan`→`Selesai`. Ownership check DI QUERY (`pelamar_id`, IDOR→404).
    async fn tandai_selesai(
        &self,
        lamaran_id: Uuid,
        pelamar_id: Uuid,
    ) -> Result<Option<Lamaran>, anyhow::Error>;
    /// P1.7: pembatalan lamaran `Diterima` oleh pemilik iklan (alasan wajib, validasi
    /// H-24 jam di application layer sebelum panggil ini). Ownership check DI QUERY.
    async fn batalkan_lamaran(
        &self,
        lamaran_id: Uuid,
        iklan_owner_id: Uuid,
        alasan: &str,
    ) -> Result<Option<Lamaran>, anyhow::Error>;
}
