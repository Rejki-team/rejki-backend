use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub title: String,
    pub body: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SendNotificationInput {
    pub recipient_id: Uuid,
    #[validate(length(min = 1, max = 255, message = "title harus 1-255 karakter"))]
    pub title: String,
    #[validate(length(min = 1, max = 1000, message = "body harus 1-1000 karakter"))]
    pub body: String,
    pub data: Option<serde_json::Value>,
}

// ── Device token ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DeviceTokenResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDeviceTokenInput {
    #[validate(length(min = 1, max = 1024, message = "token FCM harus 1-1024 karakter"))]
    pub token: String,
    #[validate(length(min = 1, max = 20))]
    pub platform: String, // android | ios | web — validated at service layer
}
