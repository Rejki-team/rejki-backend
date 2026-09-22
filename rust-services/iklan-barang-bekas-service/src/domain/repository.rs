use super::entity::{Bider, IklanBarangBekas, IklanSuspension};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AdminListParams {
    pub q: Option<String>,
    pub moderation_status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct AdminListResult {
    pub items: Vec<IklanBarangBekas>,
    pub total: i64,
}

/// Params untuk `update()` — semua field Optional (partial update).
#[derive(Debug, Clone)]
pub struct UpdateBarangBekasParams {
    pub judul: Option<String>,
    pub deskripsi: Option<String>,
    pub jenis_barang: Option<String>,
    pub jumlah: Option<i32>,
    pub lokasi_pengambilan: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub foto_urls: Option<Vec<String>>,
    pub is_active: Option<bool>,
    /// Hasil re-geocoding (F-1) bila `lokasi`/`region_id` berubah. `None` = tidak berubah
    /// ATAU geocoding gagal — kolom lama dipertahankan (COALESCE), degradasi anggun.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreateBarangBekasParams<'a> {
    pub seller_id: Uuid,
    pub judul: &'a str,
    pub deskripsi: &'a str,
    pub jenis_barang: &'a str,
    pub jumlah: i32,
    pub lokasi_pengambilan: &'a str,
    pub lokasi: Option<&'a str>,
    pub region_id: Option<&'a str>,
    pub foto_urls: &'a [String],
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[allow(async_fn_in_trait)]
pub trait IklanBarangBekasRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error>;
    /// `radius`: filter "dalam radius X km dari koordinat pengguna" (F-1, PRD §5.14.1) —
    /// `None` = tidak difilter (semua iklan aktif, seperti sebelumnya).
    async fn list(
        &self,
        limit: i64,
        offset: i64,
        radius: Option<common_geo::RadiusQuery>,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
    async fn create(
        &self,
        params: CreateBarangBekasParams<'_>,
    ) -> Result<IklanBarangBekas, anyhow::Error>;
    /// Tandai barang sebagai "sudah diambil". Hanya pemilik (seller_id).
    /// Kembalikan true bila berhasil, false bila bukan pemilik/iklan tidak ditemukan.
    async fn mark_taken(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error>;
    async fn update(
        &self,
        id: Uuid,
        seller_id: Uuid,
        params: UpdateBarangBekasParams,
    ) -> Result<Option<IklanBarangBekas>, anyhow::Error>;
    async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    async fn admin_list(&self, params: AdminListParams) -> Result<AdminListResult, anyhow::Error>;
    async fn admin_list_all(
        &self,
        params: AdminListParams,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
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
    async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error>;
    async fn is_poster_in_cooldown(&self, poster_id: Uuid) -> Result<bool, anyhow::Error>;

    /// "Iklan Saya" (Kelompok 3 Phase 4) — daftar iklan milik satu seller, entry
    /// point ke "Kelola Iklan Saya" (PRD §5.14.2). Tidak difilter availability/moderasi
    /// — pemilik tetap boleh lihat iklan sendiri yang sudah diambil/di-suspend.
    async fn list_by_seller(
        &self,
        seller_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;
    /// Batch lookup (Hazard #5, P4.11) — dipakai `list_bider_saya` untuk enrichment
    /// konteks iklan per baris Bider, satu query untuk seluruh `iklan_id` di daftar.
    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<IklanBarangBekas>, anyhow::Error>;

    // ── Bider (F-15, Kelompok 3 Phase 3) ────────────────────────────────────
    async fn find_bider_by_id(&self, id: Uuid) -> Result<Option<Bider>, anyhow::Error>;
    async fn create_bider(&self, iklan_id: Uuid, peminat_id: Uuid) -> Result<Bider, anyhow::Error>;
    /// P3.3: daftar bider untuk satu iklan (dipanggil pemilik iklan — ownership dicek di
    /// application layer via `find_by_id`, pola kembar `list_lamaran_for_iklan` Lamaran).
    async fn list_bider_for_iklan(&self, iklan_id: Uuid) -> Result<Vec<Bider>, anyhow::Error>;
    /// P3.2 validasi: cegah `peminat_id` mengajukan bid ganda (status `menunggu`) untuk
    /// `iklan_id` yang sama — pola kembar `has_conflicting_lamaran`.
    async fn has_pending_bider(
        &self,
        iklan_id: Uuid,
        peminat_id: Uuid,
    ) -> Result<bool, anyhow::Error>;
    /// P3.4: transaksi atomik (Hazard #4) — Bider `Menunggu`→`Disetujui`, Iklan→`SudahDiambil`,
    /// bider lain (bila ada, status `Menunggu`) ditandai `Withdrawn` (tidak relevan lagi).
    /// Ownership check DI QUERY (`iklan.seller_id`, IDOR→404).
    async fn setujui_bider(
        &self,
        bider_id: Uuid,
        iklan_owner_id: Uuid,
        sudah_menghubungi: bool,
    ) -> Result<Option<Bider>, anyhow::Error>;
    /// P3.5: transaksi atomik — Bider→`Withdrawn`; BILA bider yang di-withdraw sebelumnya
    /// berstatus `Disetujui`, Iklan otomatis kembali `Tersedia` (re-listing). Ownership check
    /// DI QUERY.
    async fn withdraw_bider(
        &self,
        bider_id: Uuid,
        iklan_owner_id: Uuid,
    ) -> Result<Option<Bider>, anyhow::Error>;

    /// "Bider Saya" (Kelompok 3 Phase 4) — daftar Bider milik satu peminat, dipakai
    /// Riwayat → Aktifitas → Barang Bekas (pola kembar `list_lamaran_for_pelamar`).
    async fn list_bider_for_peminat(&self, peminat_id: Uuid) -> Result<Vec<Bider>, anyhow::Error>;
}
