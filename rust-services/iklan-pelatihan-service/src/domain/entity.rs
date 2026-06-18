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

/// 7-stage lifecycle status untuk pelatihan.
/// Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D1
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PelatihanStatus {
    #[default]
    VerifikasiTertunda,
    VerifikasiDalamProses,
    VerifikasiDitolak,
    VerifikasiDiterima,
    PelatihanBelumDimulai,
    PelatihanBerjalan,
    PelatihanSelesai,
}

/// Nama status lifecycle yang disimpan di DB — jangan hardcode string literal.
pub mod status_name {
    pub const VERIFIKASI_TERTUNDA: &str = "verifikasi_tertunda";
    pub const VERIFIKASI_DALAM_PROSES: &str = "verifikasi_dalam_proses";
    pub const VERIFIKASI_DITOLAK: &str = "verifikasi_ditolak";
    pub const VERIFIKASI_DITERIMA: &str = "verifikasi_diterima";
    pub const PELATIHAN_BELUM_DIMULAI: &str = "pelatihan_belum_dimulai";
    pub const PELATIHAN_BERJALAN: &str = "pelatihan_berjalan";
    pub const PELATIHAN_SELESAI: &str = "pelatihan_selesai";
}

impl PelatihanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PelatihanStatus::VerifikasiTertunda => status_name::VERIFIKASI_TERTUNDA,
            PelatihanStatus::VerifikasiDalamProses => status_name::VERIFIKASI_DALAM_PROSES,
            PelatihanStatus::VerifikasiDitolak => status_name::VERIFIKASI_DITOLAK,
            PelatihanStatus::VerifikasiDiterima => status_name::VERIFIKASI_DITERIMA,
            PelatihanStatus::PelatihanBelumDimulai => status_name::PELATIHAN_BELUM_DIMULAI,
            PelatihanStatus::PelatihanBerjalan => status_name::PELATIHAN_BERJALAN,
            PelatihanStatus::PelatihanSelesai => status_name::PELATIHAN_SELESAI,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            status_name::VERIFIKASI_TERTUNDA => Some(PelatihanStatus::VerifikasiTertunda),
            status_name::VERIFIKASI_DALAM_PROSES => Some(PelatihanStatus::VerifikasiDalamProses),
            status_name::VERIFIKASI_DITOLAK => Some(PelatihanStatus::VerifikasiDitolak),
            status_name::VERIFIKASI_DITERIMA => Some(PelatihanStatus::VerifikasiDiterima),
            status_name::PELATIHAN_BELUM_DIMULAI => Some(PelatihanStatus::PelatihanBelumDimulai),
            status_name::PELATIHAN_BERJALAN => Some(PelatihanStatus::PelatihanBerjalan),
            status_name::PELATIHAN_SELESAI => Some(PelatihanStatus::PelatihanSelesai),
            _ => None,
        }
    }

    /// Apakah status ini sudah terminal / tidak bisa diubah lagi oleh admin?
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PelatihanStatus::VerifikasiDitolak | PelatihanStatus::PelatihanSelesai
        )
    }

    /// Apakah status ini masih bisa di-review (belum approve/reject final)?
    pub fn can_review(&self) -> bool {
        matches!(
            self,
            PelatihanStatus::VerifikasiTertunda | PelatihanStatus::VerifikasiDalamProses
        )
    }
}

impl std::fmt::Display for PelatihanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Role pembuat iklan — menentukan alur auto-approve vs review.
/// Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D2
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreatedByRole {
    #[default]
    User,
    Admin,
}

/// Nama peran yang disimpan di DB — jangan hardcode string literal.
pub mod role_name {
    pub const USER: &str = "user";
    pub const ADMIN: &str = "admin";
}

impl CreatedByRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            CreatedByRole::User => role_name::USER,
            CreatedByRole::Admin => role_name::ADMIN,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            role_name::USER => Some(CreatedByRole::User),
            role_name::ADMIN => Some(CreatedByRole::Admin),
            _ => None,
        }
    }
}

impl std::fmt::Display for CreatedByRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct IklanPelatihan {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub region_id: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
    pub status: PelatihanStatus,
    pub created_by_role: CreatedByRole,
    pub jumlah_peserta: Option<i32>,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
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

/// Status enrollment / pendaftaran peserta.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    #[default]
    Pending,
    InReview,
    Rejected,
    Approved,
}

/// Nama status enrollment untuk DB — jangan hardcode string literal.
pub mod enrollment_status_name {
    pub const PENDING: &str = "pending";
    pub const IN_REVIEW: &str = "in_review";
    pub const REJECTED: &str = "rejected";
    pub const APPROVED: &str = "approved";
}

impl EnrollmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnrollmentStatus::Pending => enrollment_status_name::PENDING,
            EnrollmentStatus::InReview => enrollment_status_name::IN_REVIEW,
            EnrollmentStatus::Rejected => enrollment_status_name::REJECTED,
            EnrollmentStatus::Approved => enrollment_status_name::APPROVED,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            enrollment_status_name::PENDING => Some(EnrollmentStatus::Pending),
            enrollment_status_name::IN_REVIEW => Some(EnrollmentStatus::InReview),
            enrollment_status_name::REJECTED => Some(EnrollmentStatus::Rejected),
            enrollment_status_name::APPROVED => Some(EnrollmentStatus::Approved),
            _ => None,
        }
    }

    pub fn can_review(&self) -> bool {
        matches!(self, EnrollmentStatus::Pending | EnrollmentStatus::InReview)
    }
}

impl std::fmt::Display for EnrollmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Pendaftaran peserta pelatihan + bukti transfer.
/// Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D3
#[derive(Debug, Clone)]
pub struct PelatihanEnrollment {
    pub id: Uuid,
    pub pelatihan_id: Uuid,
    pub user_id: Uuid,
    pub bukti_transfer_object_key: Option<String>,
    pub status: EnrollmentStatus,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Pengajuan badge / sertifikat oleh peserta.
/// Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D4
#[derive(Debug, Clone)]
pub struct PelatihanBadge {
    pub id: Uuid,
    pub pelatihan_id: Uuid,
    pub user_id: Uuid,
    pub sertifikat_object_key: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub status: EnrollmentStatus,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
