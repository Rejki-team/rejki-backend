use serde::{Deserialize, Serialize};
use validator::Validate;

// ── Register ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,

    #[validate(custom(function = "crate::application::password::validate_password_strength"))]
    pub password: String,

    /// Nomor telepon E.164 (mis. +6281234567890). Disimpan, tidak diverifikasi SMS (K8).
    #[validate(custom(function = "crate::application::password::validate_e164"))]
    pub phone: Option<String>,

    /// Persetujuan syarat & ketentuan — wajib true (US-01).
    #[serde(default)]
    #[validate(custom(function = "crate::application::password::validate_tos_accepted"))]
    pub tos_accepted: bool,

    /// Versi dokumen T&C yang disetujui (opsional; default "v1" bila kosong).
    pub tos_version: Option<String>,
}

// ── Login ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct LoginInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 1, message = "password wajib diisi"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

// ── OTP ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyOtpInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, max = 6, message = "OTP harus 6 digit"))]
    pub otp: String,
    #[validate(length(min = 1, message = "purpose wajib diisi"))]
    pub purpose: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResendOtpInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 1, message = "purpose wajib diisi"))]
    pub purpose: String,
}

// ── Refresh ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RefreshInput {
    pub refresh_token: String,
}

// ── Password recovery (US-05 / US-06) ─────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(min = 6, max = 6, message = "OTP harus 6 digit"))]
    pub otp: String,
    #[validate(custom(function = "crate::application::password::validate_password_strength"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordInput {
    #[validate(length(min = 6, max = 6, message = "OTP harus 6 digit"))]
    pub otp: String,
    #[validate(custom(function = "crate::application::password::validate_password_strength"))]
    pub new_password: String,
}

// ── Suspend (US-07, admin) ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct SuspendInput {
    /// true = permanen; false = sementara (wajib sertakan expires_at).
    #[serde(default)]
    pub permanent: bool,
    #[validate(length(min = 1, message = "alasan wajib diisi"))]
    pub reason: String,
    /// Wajib bila `permanent = false`. Diabaikan bila permanen.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}
