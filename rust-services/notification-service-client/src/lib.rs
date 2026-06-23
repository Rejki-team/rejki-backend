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

/// Token perangkat untuk push notification via FCM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceTokenInput {
    pub token: String,
    pub platform: String, // android | ios | web
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

    /// Daftarkan token perangkat untuk push notification FCM.
    async fn register_device_token(
        &self,
        user_id: Uuid,
        input: DeviceTokenInput,
    ) -> Result<DeviceToken, NotificationClientError>;

    /// Hapus token perangkat (saat logout / ganti akun).
    async fn unregister_device_token(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<(), NotificationClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationClientError {
    #[error("notification service unavailable")]
    Unavailable,
}
