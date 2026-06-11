use chrono::{DateTime, Utc};
use uuid::Uuid;

// AccountStatus dimiliki bersama lintas domain — sumber tunggal di auth-service-client.
pub use auth_service_client::AccountStatus;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub status: AccountStatus,
    pub phone: Option<String>, // ciphertext saat dari DB; plaintext setelah didekripsi
    pub tos_accepted_at: Option<DateTime<Utc>>,
    pub tos_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OtpVerification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub purpose: OtpPurpose,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtpPurpose {
    Register,
    ResetPassword,
    ChangePassword,
}

impl OtpPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpPurpose::Register => "register",
            OtpPurpose::ResetPassword => "reset_password",
            OtpPurpose::ChangePassword => "change_password",
        }
    }
}
