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

#[derive(Debug, Clone)]
pub struct IklanPekerjaan {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String, // "full_time" | "part_time" | "freelance" | "internship"
    pub foto_urls: Vec<String>,
    pub is_active: bool,
    pub moderation_status: ModerationStatus,
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
