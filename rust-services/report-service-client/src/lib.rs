/// Jenis Laporan — JALUR pembuatan aduan (bukan target-nya), PRD §6.10/§5.10,
/// keputusan final klien B-8. `LaporkanIklan`: dari halaman detail Iklan
/// Pekerjaan/Pekerja, target WAJIB. `PelaporanMasalah`: dari proses yang gagal,
/// target OPSIONAL (mis. gangguan teknis tanpa iklan terkait).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    LaporkanIklan,
    PelaporanMasalah,
}

pub mod report_type_name {
    pub const LAPORKAN_IKLAN: &str = "laporkan_iklan";
    pub const PELAPORAN_MASALAH: &str = "pelaporan_masalah";
}

impl ReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportType::LaporkanIklan => report_type_name::LAPORKAN_IKLAN,
            ReportType::PelaporanMasalah => report_type_name::PELAPORAN_MASALAH,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            report_type_name::LAPORKAN_IKLAN => Some(ReportType::LaporkanIklan),
            report_type_name::PELAPORAN_MASALAH => Some(ReportType::PelaporanMasalah),
            _ => None,
        }
    }
}

/// Jenis target aduan — iklan atau pengguna.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportTargetType {
    Iklan,
    User,
}

/// Nama kategori target yang disimpan di DB — jangan hardcode string literal.
pub mod target_type_name {
    pub const IKLAN: &str = "iklan";
    pub const USER: &str = "user";
}

impl ReportTargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportTargetType::Iklan => target_type_name::IKLAN,
            ReportTargetType::User => target_type_name::USER,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            target_type_name::IKLAN => Some(ReportTargetType::Iklan),
            target_type_name::USER => Some(ReportTargetType::User),
            _ => None,
        }
    }
}

/// Jenis iklan yang diadukan (P9.0, Kelompok 6 Q9) — `ReportTargetType::Iklan`
/// sendiri TIDAK bisa membedakan Iklan Pekerjaan/Pekerja/Barang Bekas satu
/// sama lain; field ini menutup gap itu supaya endpoint approve-and-suspend
/// (P9.1) tahu domain client mana yang harus dipanggil. `None` bila
/// `target_type=User` atau tidak diketahui.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportAdType {
    Pekerjaan,
    Pekerja,
    BarangBekas,
}

pub mod ad_type_name {
    pub const PEKERJAAN: &str = "pekerjaan";
    pub const PEKERJA: &str = "pekerja";
    pub const BARANG_BEKAS: &str = "barang_bekas";
}

impl ReportAdType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportAdType::Pekerjaan => ad_type_name::PEKERJAAN,
            ReportAdType::Pekerja => ad_type_name::PEKERJA,
            ReportAdType::BarangBekas => ad_type_name::BARANG_BEKAS,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            ad_type_name::PEKERJAAN => Some(ReportAdType::Pekerjaan),
            ad_type_name::PEKERJA => Some(ReportAdType::Pekerja),
            ad_type_name::BARANG_BEKAS => Some(ReportAdType::BarangBekas),
            _ => None,
        }
    }
}

/// Status aduan — pending → (in_review) → resolved | rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pending,
    InReview,
    Rejected,
    Resolved,
}

/// Nama status yang disimpan di DB — jangan hardcode string literal.
pub mod status_name {
    pub const PENDING: &str = "pending";
    pub const IN_REVIEW: &str = "in_review";
    pub const REJECTED: &str = "rejected";
    pub const RESOLVED: &str = "resolved";
}

impl ReportStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportStatus::Pending => status_name::PENDING,
            ReportStatus::InReview => status_name::IN_REVIEW,
            ReportStatus::Rejected => status_name::REJECTED,
            ReportStatus::Resolved => status_name::RESOLVED,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            status_name::PENDING => Some(ReportStatus::Pending),
            status_name::IN_REVIEW => Some(ReportStatus::InReview),
            status_name::REJECTED => Some(ReportStatus::Rejected),
            status_name::RESOLVED => Some(ReportStatus::Resolved),
            _ => None,
        }
    }

    /// Apakah status ini terminal (tidak bisa ditindaklanjuti lagi).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ReportStatus::Resolved | ReportStatus::Rejected)
    }
}

/// Ringkasan aduan — dipakai oleh domain lain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportSummary {
    pub id: uuid::Uuid,
    pub reporter_id: uuid::Uuid,
    pub target_type: ReportTargetType,
    pub target_id: uuid::Uuid,
    pub status: ReportStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait ReportClient: Send + Sync {
    /// Buat aduan baru oleh domain lain (in-process).
    async fn create_report(
        &self,
        reporter_id: uuid::Uuid,
        target_type: ReportTargetType,
        target_id: uuid::Uuid,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<uuid::Uuid, ReportClientError>;

    /// Cek status terbaru aduan.
    async fn get_report_status(
        &self,
        report_id: uuid::Uuid,
    ) -> Result<ReportStatus, ReportClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ReportClientError {
    #[error("report not found")]
    NotFound,
    #[error("report service unavailable")]
    Unavailable,
}
