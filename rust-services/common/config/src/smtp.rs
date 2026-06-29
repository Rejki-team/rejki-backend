use serde::Deserialize;

/// SMTP / Email configuration — semua optional (fail-open).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SmtpConfig {
    /// SMTP host, e.g., `smtp.gmail.com`.
    pub host: Option<String>,
    /// SMTP port (default: 587).
    pub port: u16,
    /// SMTP username / email.
    pub user: Option<String>,
    /// SMTP password / app password.
    pub pass: Option<String>,
    /// From address for outgoing emails, e.g., `Rejki <noreply@rejki.id>`.
    pub email_from: Option<String>,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: None,
            port: 587,
            user: None,
            pass: None,
            email_from: None,
        }
    }
}
