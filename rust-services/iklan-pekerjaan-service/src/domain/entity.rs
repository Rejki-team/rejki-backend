use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Status moderasi iklan (state machine admin).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus {
    #[default]
    Active,
    SuspendedTemp,
    SuspendedPermanent,
}

impl ModerationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationStatus::Active => "active",
            ModerationStatus::SuspendedTemp => "suspended_temp",
            ModerationStatus::SuspendedPermanent => "suspended_permanent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ModerationStatus::Active),
            "suspended_temp" => Some(ModerationStatus::SuspendedTemp),
            "suspended_permanent" => Some(ModerationStatus::SuspendedPermanent),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModerationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Status siklus KERJA (bukan moderasi/visibilitas — lihat `ModerationStatus`/`is_active`
/// yang tetap terpisah). Kelompok 3 Phase 1 (F-3), match Bab 9 PRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PekerjaanStatus {
    #[default]
    Tersedia,
    SedangDikerjakan,
    Selesai,
}

pub mod pekerjaan_status_name {
    pub const TERSEDIA: &str = "tersedia";
    pub const SEDANG_DIKERJAKAN: &str = "sedang_dikerjakan";
    pub const SELESAI: &str = "selesai";
}

impl PekerjaanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PekerjaanStatus::Tersedia => pekerjaan_status_name::TERSEDIA,
            PekerjaanStatus::SedangDikerjakan => pekerjaan_status_name::SEDANG_DIKERJAKAN,
            PekerjaanStatus::Selesai => pekerjaan_status_name::SELESAI,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            pekerjaan_status_name::TERSEDIA => Some(PekerjaanStatus::Tersedia),
            pekerjaan_status_name::SEDANG_DIKERJAKAN => Some(PekerjaanStatus::SedangDikerjakan),
            pekerjaan_status_name::SELESAI => Some(PekerjaanStatus::Selesai),
            _ => None,
        }
    }
}

impl std::fmt::Display for PekerjaanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct IklanPekerjaan {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String, // "full_time" | "part_time" | "freelance" | "internship"
    /// Jam kerja (F-5, Kelompok 6 P7.1) — teks bebas mis. "08:00-17:00". Nullable:
    /// iklan lama historis.
    pub jam_kerja: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    /// Status siklus kerja (F-3, Kelompok 3 Phase 1) — Tersedia saat dibuat, berubah
    /// otomatis lewat alur Lamaran (`mulai_bekerja`/`tandai_selesai`), TIDAK lewat PATCH biasa.
    pub status: PekerjaanStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    // Koordinat hasil geocoding `lokasi`/`region_id` (F-1) — dipakai filter radius 2km.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Riwayat suspend iklan (audit trail).
#[derive(Debug, Clone)]
pub struct IklanSuspension {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub is_permanent: bool,
    pub reason: String,
    pub evidence_object_key: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Status Lamaran — match Bab 9 PRD persis (F-3, Kelompok 3 Phase 1). `Ditolak` juga
/// dipakai untuk pembatalan lamaran yang sudah `Diterima` oleh pemilik iklan (P1.7) —
/// PRD Bab 9 hanya mendaftar 5 status, tidak ada status ke-6 "dibatalkan" terpisah;
/// `alasan_batal` di entity `Lamaran` membedakan konteksnya bila perlu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LamaranStatus {
    #[default]
    Diajukan,
    Diterima,
    Ditolak,
    Proses,
    Selesai,
}

pub mod lamaran_status_name {
    pub const DIAJUKAN: &str = "diajukan";
    pub const DITERIMA: &str = "diterima";
    pub const DITOLAK: &str = "ditolak";
    pub const PROSES: &str = "proses";
    pub const SELESAI: &str = "selesai";
}

impl LamaranStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LamaranStatus::Diajukan => lamaran_status_name::DIAJUKAN,
            LamaranStatus::Diterima => lamaran_status_name::DITERIMA,
            LamaranStatus::Ditolak => lamaran_status_name::DITOLAK,
            LamaranStatus::Proses => lamaran_status_name::PROSES,
            LamaranStatus::Selesai => lamaran_status_name::SELESAI,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            lamaran_status_name::DIAJUKAN => Some(LamaranStatus::Diajukan),
            lamaran_status_name::DITERIMA => Some(LamaranStatus::Diterima),
            lamaran_status_name::DITOLAK => Some(LamaranStatus::Ditolak),
            lamaran_status_name::PROSES => Some(LamaranStatus::Proses),
            lamaran_status_name::SELESAI => Some(LamaranStatus::Selesai),
            _ => None,
        }
    }

    /// Status terminal — tidak bisa ditransisikan lagi.
    pub fn is_terminal(&self) -> bool {
        matches!(self, LamaranStatus::Ditolak | LamaranStatus::Selesai)
    }
}

impl std::fmt::Display for LamaranStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lamaran pekerjaan — alur lamar-kerja penuh (F-3, PRD §5.11.3-5.11.5).
#[derive(Debug, Clone)]
pub struct Lamaran {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub pelamar_id: Uuid,
    pub status: LamaranStatus,
    /// Tanggal & jam kerja yang diajukan/disepakati (default dari iklan atau hasil
    /// kesepakatan pelamar-pemberi kerja — PRD §5.11.3).
    pub tanggal: chrono::NaiveDate,
    pub jam_mulai: chrono::NaiveTime,
    pub jam_akhir: chrono::NaiveTime,
    /// Jumlah slot kuota pekerja yang diambil lamaran ini (default 1).
    pub kuota_diambil: i32,
    /// Alasan wajib saat pemilik iklan membatalkan lamaran yang sudah Diterima (P1.7).
    pub alasan_batal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
