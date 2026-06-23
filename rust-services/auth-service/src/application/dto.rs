use serde::{Deserialize, Serialize};
use uuid::Uuid;
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
    #[validate(length(
        min = 1,
        max = 128,
        message = "password wajib diisi (maks 128 karakter)"
    ))]
    pub password: String,
}

// ── Admin Login ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct AdminLoginInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(length(
        min = 1,
        max = 128,
        message = "password wajib diisi (maks 128 karakter)"
    ))]
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
    #[validate(custom(
        function = "crate::domain::entity::OtpPurpose::validate_purpose",
        message = "purpose harus salah satu: register, reset_password, change_password"
    ))]
    pub purpose: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResendOtpInput {
    #[validate(email(message = "email tidak valid"))]
    pub email: String,
    #[validate(custom(
        function = "crate::domain::entity::OtpPurpose::validate_purpose",
        message = "purpose harus salah satu: register, reset_password, change_password"
    ))]
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
    /// Object key bukti penangguhan (gambar/dokumen, maks 5MB, US-07/Q1).
    /// Admin wajib mengunggah bukti via endpoint evidence terlebih dahulu.
    pub evidence_object_key: Option<String>,
}

// ── Bukti penangguhan (US-07 / Q1) ──────────────────────────────────────────────

/// Permintaan presigned URL untuk mengunggah bukti penangguhan (gambar/PDF, maks 5MB).
#[derive(Debug, Deserialize, Validate)]
pub struct SuspendEvidenceRequest {
    #[validate(length(min = 1))]
    pub mime: String,
    pub size_bytes: u64,
}

// ── Bulk suspend (extend-user-suspension-bulk-purge) ───────────────────────

/// Maksimum jumlah pengguna yang dapat di-suspend dalam satu permintaan (D6).
pub const MAX_BULK_SUSPEND_USERS: usize = 100;

/// Kompilasi-time assertion: validasi DTO harus sinkron dengan MAX_BULK_SUSPEND_USERS.
const _: () = assert!(
    MAX_BULK_SUSPEND_USERS == 100,
    "Sinkronkan MAX_BULK_SUSPEND_USERS dengan BulkSuspendInput.user_ids length validator"
);

/// Input suspend massal — menerima daftar user_id + parameter suspend.
#[derive(Debug, Deserialize, Validate)]
pub struct BulkSuspendInput {
    /// Daftar user_id yang akan di-suspend (1–100, lihat MAX_BULK_SUSPEND_USERS).
    #[validate(length(min = 1, max = 100, message = "wajib 1-100 user"))]
    pub user_ids: Vec<Uuid>,
    /// true = permanen; false = sementara (wajib sertakan expires_at).
    #[serde(default)]
    pub permanent: bool,
    #[validate(length(min = 1, message = "alasan wajib diisi"))]
    pub reason: String,
    /// Wajib bila `permanent = false`. Diabaikan bila permanen.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Object key bukti penangguhan (gambar/dokumen, maks 5MB).
    pub evidence_object_key: Option<String>,
}

/// Hasil per-user dari suspend massal — partial-success (D2).
#[derive(Debug, Serialize)]
pub struct BulkSuspendResultItem {
    pub user_id: Uuid,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Respons suspend massal — array per-item (konsisten dengan SuspendResponse iklan).
#[derive(Debug, Serialize)]
pub struct BulkSuspendResponse {
    pub results: Vec<BulkSuspendResultItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn register_input_validates() {
        assert!(RegisterInput {
            email: "user@example.com".into(),
            password: "Strong1!".into(),
            phone: Some("+6281234567890".into()),
            tos_accepted: true,
            tos_version: Some("v1".into()),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn register_input_rejects_invalid_email() {
        assert!(RegisterInput {
            email: "not-email".into(),
            password: "Strong1!".into(),
            phone: None,
            tos_accepted: true,
            tos_version: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn register_input_requires_tos_accepted() {
        assert!(RegisterInput {
            email: "user@example.com".into(),
            password: "Strong1!".into(),
            phone: None,
            tos_accepted: false,
            tos_version: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn register_input_rejects_weak_password() {
        assert!(RegisterInput {
            email: "user@example.com".into(),
            password: "short".into(),
            phone: None,
            tos_accepted: true,
            tos_version: None,
        }
        .validate()
        .is_err());
    }

    #[test]
    fn login_input_validates() {
        assert!(LoginInput {
            email: "user@example.com".into(),
            password: "somepass".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn login_input_rejects_empty_password() {
        assert!(LoginInput {
            email: "user@example.com".into(),
            password: "".into(),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn login_input_rejects_too_long_password() {
        assert!(LoginInput {
            email: "user@example.com".into(),
            password: "a".repeat(129),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn admin_login_input_validates() {
        assert!(AdminLoginInput {
            email: "admin@rejki.id".into(),
            password: "adminpass".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn admin_login_input_rejects_too_long_password() {
        assert!(AdminLoginInput {
            email: "admin@rejki.id".into(),
            password: "a".repeat(129),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn verify_otp_input_validates() {
        assert!(VerifyOtpInput {
            email: "user@example.com".into(),
            otp: "123456".into(),
            purpose: "register".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn verify_otp_input_rejects_short_otp() {
        assert!(VerifyOtpInput {
            email: "user@example.com".into(),
            otp: "12345".into(),
            purpose: "register".into(),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn verify_otp_input_rejects_empty_purpose() {
        assert!(VerifyOtpInput {
            email: "user@example.com".into(),
            otp: "123456".into(),
            purpose: "".into(),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn reset_password_input_validates() {
        assert!(ResetPasswordInput {
            email: "user@example.com".into(),
            otp: "123456".into(),
            new_password: "NewStrong1!".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn change_password_input_validates() {
        assert!(ChangePasswordInput {
            otp: "123456".into(),
            new_password: "NewStrong1!".into(),
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn bulk_suspend_input_validates() {
        let input = BulkSuspendInput {
            user_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            permanent: true,
            reason: "TOS violation".into(),
            expires_at: None,
            evidence_object_key: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn bulk_suspend_input_rejects_empty_user_ids() {
        let input = BulkSuspendInput {
            user_ids: vec![],
            permanent: true,
            reason: "TOS violation".into(),
            expires_at: None,
            evidence_object_key: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn bulk_suspend_input_rejects_empty_reason() {
        let input = BulkSuspendInput {
            user_ids: vec![Uuid::now_v7()],
            permanent: true,
            reason: "".into(),
            expires_at: None,
            evidence_object_key: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn max_bulk_suspend_users_constant_is_100() {
        assert_eq!(MAX_BULK_SUSPEND_USERS, 100);
    }

    #[test]
    fn verify_otp_input_rejects_invalid_purpose() {
        let input = VerifyOtpInput {
            email: "user@example.com".into(),
            otp: "123456".into(),
            purpose: "hack".into(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn resend_otp_input_rejects_invalid_purpose() {
        let input = ResendOtpInput {
            email: "user@example.com".into(),
            purpose: "attack".into(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn verify_otp_input_accepts_valid_purposes() {
        for purpose in &["register", "reset_password", "change_password"] {
            let input = VerifyOtpInput {
                email: "user@example.com".into(),
                otp: "123456".into(),
                purpose: (*purpose).into(),
            };
            assert!(input.validate().is_ok(), "purpose '{purpose}' harus valid");
        }
    }
}
