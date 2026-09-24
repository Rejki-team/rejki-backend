use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub user_a: Uuid,
    pub user_b: Uuid,
    pub ended_at: Option<DateTime<Utc>>,
    pub related_ad_type: Option<String>,
    pub related_ad_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content_type: String,
    pub content: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub photo_object_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Input kirim pesan — satu dari tiga jenis (F-19, PRD §5.9). Divalidasi manual di
/// `ChatService::send_message` (bukan `#[derive(Validate)]` — validator tidak
/// mendukung enum bertag secara native).
#[derive(Debug, Deserialize)]
#[serde(tag = "content_type", rename_all = "snake_case")]
pub enum SendMessageInput {
    Text { content: String },
    Location { lat: f64, lng: f64 },
    Photo { photo_object_key: String },
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub limit: Option<i64>,
    pub before_id: Option<Uuid>,
    pub after_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct MessageListResponse {
    pub messages: Vec<MessageResponse>,
}

/// Izin unggah foto chat (P4.2) — mirror `storage_service_client::UploadPermission`.
#[derive(Debug, Serialize)]
pub struct PhotoUploadPermissionResponse {
    pub presigned_url: String,
    pub object_key: String,
}

/// Satu item "Halaman daftar percakapan" (P4.10, F-18, PRD §5.9). `other_user` bisa
/// `null` bila `UserClient` tidak terpasang atau profil tidak ditemukan — degradasi
/// anggun (mobile tetap tampilkan conversation, tanpa nama/foto).
#[derive(Debug, Serialize)]
pub struct ConversationListItemResponse {
    pub id: Uuid,
    pub other_user: Option<user_service_client::UserSummary>,
    pub last_message: Option<MessageResponse>,
    pub has_unread: bool,
    pub ended_at: Option<DateTime<Utc>>,
    pub related_ad_type: Option<String>,
    pub related_ad_id: Option<Uuid>,
}
