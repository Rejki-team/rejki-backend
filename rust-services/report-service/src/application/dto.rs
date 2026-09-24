use chrono::{DateTime, NaiveDate, Utc};
use report_service_client::{ReportStatus, ReportTargetType, ReportType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ══════════════════════════════════════════════════════════════════════════
// Response
// ══════════════════════════════════════════════════════════════════════════

/// Data demografis pelapor (P1.3, PRD §6.10) — `None` bila profil pelapor tidak
/// ditemukan (degradasi anggun, bukan error). **Catatan jujur**: PRD meminta
/// "Tempat Tanggal Lahir" tapi backend (`user-service`) hanya menyimpan tanggal
/// lahir, TANPA field tempat lahir — dicatat sebagai gap, di luar scope P1.3
/// (butuh field baru di form onboarding user-service, bukan sekadar expose data
/// yang sudah ada). `country` di sini adalah `country_code` mentah (mis. "ID"),
/// bukan nama negara penuh — tidak ada layanan resolve nama negara di backend ini.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ReporterDemographics {
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub address_line: Option<String>,
    pub village_name: Option<String>,
    pub district_name: Option<String>,
    pub regency_name: Option<String>,
    pub province_name: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub report_type: ReportType,
    pub target_type: Option<ReportTargetType>,
    pub target_id: Option<Uuid>,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    pub status: ReportStatus,
    pub action_note: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub due_date: DateTime<Utc>,
    pub is_overdue: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// `None` di listing biasa (mahal untuk semua baris) — diisi lewat batch
    /// enrichment terpisah oleh service layer (Hazard #5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporter_demographics: Option<ReporterDemographics>,
}

/// Response `POST /reports/pelaporan-masalah` (P3.0 — ditemukan sebelumnya
/// `request_upload` dipanggil tapi `presigned_url` tidak pernah dikembalikan ke
/// klien, sehingga bukti gambar tidak mungkin benar-benar terunggah). Pola sama
/// `UploadPermission` di `user-service::request_avatar_upload` — `presigned_url`
/// HANYA relevan untuk create, TIDAK ditambahkan ke `ReportResponse` umum yang
/// dipakai list/detail (di sana sudah kedaluwarsa/tidak relevan).
#[derive(Debug, Serialize)]
pub struct CreatePelaporanMasalahResponse {
    #[serde(flatten)]
    pub report: ReportResponse,
    pub presigned_url: String,
}

/// Detail aduan untuk admin — termasuk presigned read URL bukti + demografis pelapor.
#[derive(Debug, Serialize)]
pub struct ReportDetailResponse {
    pub id: Uuid,
    pub reporter_id: Uuid,
    pub report_type: ReportType,
    pub target_type: Option<ReportTargetType>,
    pub target_id: Option<Uuid>,
    pub keterangan: String,
    pub evidence_object_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_read_url: Option<String>,
    pub status: ReportStatus,
    pub action_note: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub due_date: DateTime<Utc>,
    pub is_overdue: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reporter_demographics: ReporterDemographics,
}

// ══════════════════════════════════════════════════════════════════════════
// Inputs
// ══════════════════════════════════════════════════════════════════════════

/// Jalur 1: "Laporkan Iklan" — dari halaman detail Iklan Pekerjaan/Pekerja
/// (PRD §5.10). Target WAJIB, alasan 50-255 karakter, TANPA bukti foto.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateLaporkanIklanInput {
    #[validate(length(min = 1, message = "target_type wajib diisi"))]
    pub target_type: String,
    pub target_id: Uuid,
    /// Jenis iklan (P9.0, Kelompok 6 Q9) — `pekerjaan`/`pekerja`/`barang_bekas`.
    /// Opsional: `None` bila mobile lama belum kirim (fail-open, bukan wajib —
    /// menghindari 422 di klien lama yang belum update).
    pub target_ad_type: Option<String>,
    #[validate(length(
        min = 50,
        max = 255,
        message = "alasan pelaporan wajib 50-255 karakter"
    ))]
    pub keterangan: String,
}

/// Jalur 2: "Pelaporan Masalah" — dari proses yang gagal (PRD §5.10). Target
/// OPSIONAL, deskripsi tanpa batas karakter khusus, bukti foto WAJIB maks 300KB.
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePelaporanMasalahInput {
    /// ID iklan — opsional, terisi otomatis oleh klien bila tersedia.
    pub target_id: Option<Uuid>,
    #[validate(length(min = 1, message = "deskripsi masalah wajib diisi"))]
    pub keterangan: String,
    #[validate(length(min = 1, message = "bukti gambar wajib diunggah"))]
    pub mime: String,
    #[validate(range(min = 1, max = 300_000, message = "bukti gambar maksimal 300 KB"))]
    pub size_bytes: u64,
}

/// Query params untuk listing admin.
#[derive(Debug, Deserialize)]
pub struct ReportListQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    /// Filter Jenis Laporan (fondasi filter web Phase 2, PRD §6.10).
    pub report_type: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Input tindak lanjut admin.
#[derive(Debug, Deserialize, Validate)]
pub struct ReviewReportInput {
    pub approved: bool,
    #[validate(length(
        min = 1,
        message = "action_note wajib diisi — jelaskan tindakan yang diambil"
    ))]
    pub action_note: String,
}

/// Endpoint terintegrasi "Terima & Suspend" (P9.1, Kelompok 6 Q9) — menyetujui
/// aduan SEKALIGUS men-suspend target dalam 1 aksi. `reason` pre-filled dari
/// `keterangan` laporan di sisi web (bisa diedit admin) — bukan validasi
/// backend, murni UX. Kontrak field SAMA PERSIS `SuspendIklanPayload`/
/// `SuspendPenggunaPayload` existing di web (`is_permanent` + `expires_at`
/// opsional terpisah — BUKAN `expires_at=None` berarti permanen; UI existing
/// tidak pernah mengirim tanggal spesifik, hanya toggle sementara/permanen).
#[derive(Debug, Deserialize, Validate)]
pub struct ApproveAndSuspendInput {
    #[validate(length(min = 10, message = "alasan suspend wajib diisi"))]
    pub reason: String,
    pub is_permanent: bool,
    pub expires_at: Option<DateTime<Utc>>,
}
