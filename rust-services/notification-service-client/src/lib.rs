use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
}

/// Email transaksional (mis. OTP). Dikirim via kanal email (SMTP) oleh consumer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait::async_trait]
pub trait NotificationClient: Send + Sync {
    async fn send(
        &self,
        recipient_id: Uuid,
        payload: NotificationPayload,
    ) -> Result<(), NotificationClientError>;
    async fn send_bulk(
        &self,
        recipient_ids: Vec<Uuid>,
        payload: NotificationPayload,
    ) -> Result<(), NotificationClientError>;

    /// Kirim email transaksional (mis. OTP) via kanal email.
    async fn send_email(&self, message: EmailMessage) -> Result<(), NotificationClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationClientError {
    #[error("notification service unavailable")]
    Unavailable,
}
