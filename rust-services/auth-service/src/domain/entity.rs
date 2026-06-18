use chrono::{DateTime, Utc};
use uuid::Uuid;

// OtpPurpose tidak memerlukan struct OtpVerification karena semua data OTP
// dikelola via raw SQL di PgAuthRepository (save_otp / consume_otp / bump_otp_attempts).

// AccountStatus dimiliki bersama lintas domain — sumber tunggal di auth-service-client.
pub use auth_service_client::AccountStatus;
pub use auth_service_client::Role;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub status: AccountStatus,
    pub role: Role,
    pub phone: Option<String>, // ciphertext saat dari DB; plaintext setelah didekripsi
    pub tos_accepted_at: Option<DateTime<Utc>>,
    pub tos_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

    /// Validasi string dari DTO — menerima "register", "reset_password", "change_password".
    pub fn validate_purpose(value: &str) -> Result<(), validator::ValidationError> {
        if matches!(value, "register" | "reset_password" | "change_password") {
            Ok(())
        } else {
            let mut err = validator::ValidationError::new("invalid_purpose");
            err.message =
                Some("purpose harus salah satu: register, reset_password, change_password".into());
            Err(err)
        }
    }
}
