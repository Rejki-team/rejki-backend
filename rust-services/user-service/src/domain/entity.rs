use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub id: Uuid,
    pub auth_id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    // KYC fields
    pub nik_encrypted: Option<Vec<u8>>,
    pub nik_last4: Option<String>,
    pub education_level: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<chrono::NaiveDate>,
    pub address_line: Option<String>,
    pub country_code: String,
    pub province_id: Option<String>,
    pub regency_id: Option<String>,
    pub district_id: Option<String>,
    pub village_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct KycSubmission {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub status: KycSubmissionStatus,
    pub ktp_object_key: Option<String>,
    pub selfie_object_key: Option<String>,
    pub reviewed_by: Option<Uuid>,
    pub review_note: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KycSubmissionStatus {
    Pending,
    Approved,
    Rejected,
}

impl KycSubmissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            KycSubmissionStatus::Pending => "pending",
            KycSubmissionStatus::Approved => "approved",
            KycSubmissionStatus::Rejected => "rejected",
        }
    }
}

impl std::str::FromStr for KycSubmissionStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(KycSubmissionStatus::Pending),
            "approved" => Ok(KycSubmissionStatus::Approved),
            "rejected" => Ok(KycSubmissionStatus::Rejected),
            _ => Err(()),
        }
    }
}

/// Jenis akses dokumen yang dicatat di audit trail (Q2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentAccessAction {
    /// Presigned upload URL diterbitkan (dokumen akan diunggah).
    UploadIssued,
    /// Dokumen di-commit & di-link ke submission (magic bytes terverifikasi).
    Commit,
    /// Presigned read URL diterbitkan (dokumen dibuka/dilihat).
    ReadIssued,
}

impl DocumentAccessAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentAccessAction::UploadIssued => "upload_issued",
            DocumentAccessAction::Commit => "commit",
            DocumentAccessAction::ReadIssued => "read_issued",
        }
    }
}
