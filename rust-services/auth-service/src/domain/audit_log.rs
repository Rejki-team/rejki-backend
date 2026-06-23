use std::net::IpAddr;

use uuid::Uuid;

/// Konteks request untuk audit log — diekstrak dari header HTTP.
/// Tidak menyimpan data sensitif (password, token, body request).
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

/// Event autentikasi yang dicatat ke tabel `auth.audit_log`.
/// Setiap variant membawa metadata spesifik event yang disimpan sebagai JSONB.
#[derive(Debug, Clone)]
pub enum AuditEvent {
    /// Login berhasil — user_id selalu Some.
    LoginSuccess,
    /// Login gagal — reason: "wrong password", "account not verified", "account suspended", dsb.
    LoginFailed { reason: String },
    /// Admin login berhasil.
    AdminLoginSuccess,
    /// Admin login gagal.
    AdminLoginFailed { reason: String },
    /// Logout (refresh token dicabut).
    Logout,
    /// Token refresh berhasil (rotasi refresh token).
    TokenRefresh,
    /// OTP dikirim ke email user.
    OtpSent { purpose: String },
    /// Password diubah oleh user terautentikasi (change_password).
    PasswordChanged,
    /// Password di-reset lewat forgot-password (reset_password).
    PasswordReset,
    /// Akun di-suspend (single, lewat admin).
    AccountSuspended {
        admin_id: Uuid,
        reason: String,
        permanent: bool,
    },
    /// Bulk suspend akun.
    AccountSuspendedBulk {
        admin_id: Uuid,
        count: usize,
        permanent: bool,
    },
}

impl AuditEvent {
    /// Nama event sebagai string — disimpan di kolom `event` (TEXT).
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEvent::LoginSuccess => "login_success",
            AuditEvent::LoginFailed { .. } => "login_failed",
            AuditEvent::AdminLoginSuccess => "admin_login_success",
            AuditEvent::AdminLoginFailed { .. } => "admin_login_failed",
            AuditEvent::Logout => "logout",
            AuditEvent::TokenRefresh => "token_refresh",
            AuditEvent::OtpSent { .. } => "otp_sent",
            AuditEvent::PasswordChanged => "password_changed",
            AuditEvent::PasswordReset => "password_reset",
            AuditEvent::AccountSuspended { .. } => "account_suspended",
            AuditEvent::AccountSuspendedBulk { .. } => "account_suspended_bulk",
        }
    }

    /// Metadata tambahan per event sebagai JSONB.
    /// Tidak menyimpan password/token.
    pub fn metadata(&self) -> Option<serde_json::Value> {
        match self {
            AuditEvent::LoginSuccess
            | AuditEvent::Logout
            | AuditEvent::TokenRefresh
            | AuditEvent::PasswordChanged
            | AuditEvent::PasswordReset
            | AuditEvent::AdminLoginSuccess => None,
            AuditEvent::LoginFailed { reason } => Some(serde_json::json!({ "reason": reason })),
            AuditEvent::AdminLoginFailed { reason } => {
                Some(serde_json::json!({ "reason": reason }))
            }
            AuditEvent::OtpSent { purpose } => Some(serde_json::json!({ "purpose": purpose })),
            AuditEvent::AccountSuspended {
                admin_id,
                reason,
                permanent,
            } => Some(serde_json::json!({
                "admin_id": admin_id.to_string(),
                "reason": reason,
                "permanent": permanent,
            })),
            AuditEvent::AccountSuspendedBulk {
                admin_id,
                count,
                permanent,
            } => Some(serde_json::json!({
                "admin_id": admin_id.to_string(),
                "count": count,
                "permanent": permanent,
            })),
        }
    }
}

// `async fn` di trait untuk `AuditLogRepository` (dyn-compatible via async-trait).
// Digunakan sebagai trait object (`Arc<dyn AuditLogRepository>`) untuk injeksi ke AuthService.
#[async_trait::async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// Catat satu event audit ke tabel `auth.audit_log`.
    /// `user_id` boleh None untuk event seperti failed login (user tidak dikenal).
    async fn log(
        &self,
        user_id: Option<Uuid>,
        event: AuditEvent,
        context: AuditContext,
    ) -> Result<(), anyhow::Error>;
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_event_as_str_all_variants_unique() {
        let variants: Vec<&'static str> = vec![
            AuditEvent::LoginSuccess.as_str(),
            AuditEvent::LoginFailed {
                reason: "test".into(),
            }
            .as_str(),
            AuditEvent::AdminLoginSuccess.as_str(),
            AuditEvent::AdminLoginFailed {
                reason: "test".into(),
            }
            .as_str(),
            AuditEvent::Logout.as_str(),
            AuditEvent::TokenRefresh.as_str(),
            AuditEvent::OtpSent {
                purpose: "register".into(),
            }
            .as_str(),
            AuditEvent::PasswordChanged.as_str(),
            AuditEvent::PasswordReset.as_str(),
            AuditEvent::AccountSuspended {
                admin_id: Uuid::nil(),
                reason: "test".into(),
                permanent: true,
            }
            .as_str(),
            AuditEvent::AccountSuspendedBulk {
                admin_id: Uuid::nil(),
                count: 5,
                permanent: false,
            }
            .as_str(),
        ];

        // Semua variant harus punya string unik
        let mut seen = std::collections::HashSet::new();
        for v in &variants {
            assert!(seen.insert(*v), "Duplicate event string: '{}'", v);
        }
        assert_eq!(variants.len(), 11, "Harus ada 11 variant event");
    }

    #[test]
    fn audit_event_metadata_login_failed_contains_reason() {
        let event = AuditEvent::LoginFailed {
            reason: "wrong password".into(),
        };
        let meta = event.metadata().expect("LoginFailed harus punya metadata");
        assert_eq!(meta["reason"], "wrong password");
    }

    #[test]
    fn audit_event_metadata_login_success_is_none() {
        let event = AuditEvent::LoginSuccess;
        assert!(event.metadata().is_none());
    }

    #[test]
    fn audit_event_metadata_account_suspended_contains_fields() {
        let admin_id = Uuid::now_v7();
        let event = AuditEvent::AccountSuspended {
            admin_id,
            reason: "TOS violation".into(),
            permanent: true,
        };
        let meta = event
            .metadata()
            .expect("AccountSuspended harus punya metadata");
        assert_eq!(meta["admin_id"], admin_id.to_string());
        assert_eq!(meta["reason"], "TOS violation");
        assert_eq!(meta["permanent"], true);
    }

    #[test]
    fn audit_context_debug_does_not_panic() {
        let ctx = AuditContext {
            ip_address: Some("192.168.1.1".parse().unwrap()),
            user_agent: Some("Mozilla/5.0".into()),
            request_id: Some("req-001".into()),
        };
        // Debug print tidak panic
        let _ = format!("{:?}", ctx);
    }
}
