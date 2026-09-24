use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::entity::{LamaranStatus, ModerationStatus, PekerjaanStatus};

#[derive(Debug, Serialize)]
pub struct IklanPekerjaanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub jam_kerja: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub status: PekerjaanStatus,
    pub created_at: DateTime<Utc>,
}

/// Admin response – includes deleted_at for moderation context.
#[derive(Debug, Serialize)]
pub struct AdminIklanPekerjaanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub jam_kerja: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub status: PekerjaanStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Lamaran (F-3, Kelompok 3 Phase 1) ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LamaranResponse {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub pelamar_id: Uuid,
    pub status: LamaranStatus,
    pub tanggal: NaiveDate,
    pub jam_mulai: NaiveTime,
    pub jam_akhir: NaiveTime,
    pub kuota_diambil: i32,
    pub alasan_batal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// P1.10 enrichment (Kelompok 3 Phase 2, F-4 PRD §5.11.5): `list_lamaran_for_iklan`
/// (dipakai halaman "Kelola Pelamar" pemilik iklan) melengkapi setiap baris dengan
/// ringkasan Iklan Pekerja aktif milik pelamar (nama, kode iklan) — di-batch di
/// service layer (`get_active_summaries_for_posters`), bukan N+1 per baris.
/// `pelamar_*` bernilai `None` bila pelamar tidak/belum punya Iklan Pekerja aktif
/// (mis. sudah dihapus setelah melamar) — degradasi anggun, bukan error.
#[derive(Debug, Serialize)]
pub struct LamaranWithPelamarResponse {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub pelamar_id: Uuid,
    pub status: LamaranStatus,
    pub tanggal: NaiveDate,
    pub jam_mulai: NaiveTime,
    pub jam_akhir: NaiveTime,
    pub kuota_diambil: i32,
    pub alasan_batal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pelamar_nama: Option<String>,
    pub pelamar_iklan_pekerja_id: Option<Uuid>,
    pub pelamar_keahlian: Option<Vec<String>>,
    pub pelamar_foto_url: Option<String>,
}

/// Enrichment simetris untuk `list_lamaran_for_pelamar` (Kelompok 3 Phase 2, PRD
/// §5.11.4 "Riwayat Aktifitas Pelamar" — butuh judul/upah/jenis pekerjaan/alamat
/// iklan per baris). Batch lookup via `IklanPekerjaanRepository::find_by_ids`
/// (Hazard #5). `iklan_*` bernilai `None` hanya bila data tidak konsisten
/// (seharusnya tidak terjadi — FK implisit via `iklan_id`).
#[derive(Debug, Serialize)]
pub struct LamaranWithIklanResponse {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub pelamar_id: Uuid,
    pub status: LamaranStatus,
    pub tanggal: NaiveDate,
    pub jam_mulai: NaiveTime,
    pub jam_akhir: NaiveTime,
    pub kuota_diambil: i32,
    pub alasan_batal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub iklan_judul: Option<String>,
    pub iklan_perusahaan: Option<String>,
    pub iklan_gaji_min: Option<i64>,
    pub iklan_gaji_max: Option<i64>,
    pub iklan_tipe: Option<String>,
    pub iklan_lokasi: Option<String>,
    /// P6.4 (Kelompok 3 Fase 6): pemilik iklan — dibutuhkan mobile sebagai
    /// `dinilai_id` saat submit rating arah `pelamar_ke_pemberi_kerja` (F-17,
    /// PRD §5.15). Dari `iklan_map` yang sama, tanpa query tambahan.
    pub iklan_poster_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LamarInput {
    pub tanggal: NaiveDate,
    pub jam_mulai: NaiveTime,
    pub jam_akhir: NaiveTime,
    #[validate(range(min = 1))]
    pub kuota_diambil: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewLamaranInput {
    pub approved: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BatalkanLamaranInput {
    #[validate(length(min = 10))]
    pub alasan: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct MulaiBekerjaInput {
    #[validate(range(min = -90.0, max = 90.0))]
    pub latitude: f64,
    #[validate(range(min = -180.0, max = 180.0))]
    pub longitude: f64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPekerjaanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 2, max = 200))]
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub jam_kerja: Option<String>,
    pub foto_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Validate, Deserialize)]
pub struct UpdatePekerjaanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: Option<String>,
    #[validate(length(min = 2, max = 200))]
    pub perusahaan: Option<String>,
    #[validate(length(min = 1))]
    pub deskripsi: Option<String>,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: Option<String>, // full_time, part_time, freelance, internship
    pub jam_kerja: Option<String>,
    pub foto_urls: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Koordinat pengguna (F-1) — filter radius aktif hanya bila `latitude`+`longitude` diisi
    /// keduanya. Nama key sesuai kontrak mobile existing (`job_query_params.dart`).
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

// ── Admin query ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AdminListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuspendEvidenceInput {
    pub mime: String,
    #[validate(range(min = 1, max = 5242880))]
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuspendInput {
    pub iklan_ids: Vec<Uuid>,
    pub is_permanent: bool,
    #[validate(length(min = 10))]
    pub reason: String,
    pub evidence_object_key: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SuspendResultItem {
    pub iklan_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuspendResponse {
    pub results: Vec<SuspendResultItem>,
}
