use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: Uuid,
    pub user_a: Uuid,
    pub user_b: Uuid,
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub related_ad_type: Option<String>,
    pub related_ad_id: Option<Uuid>,
    /// Read-tracking (P4.11, F-18) — pesan terakhir yang sudah dibaca `user_a`/`user_b`.
    /// Kolom langsung (bukan tabel participant terpisah) — konsisten `user_a`/`user_b`
    /// denormalized yang sudah ada (skema chat memang tepat 2 pihak per conversation).
    pub user_a_last_read_message_id: Option<Uuid>,
    pub user_b_last_read_message_id: Option<Uuid>,
}

/// Jenis konten pesan (F-19, PRD §5.9) — teks bebas, bagikan lokasi, atau kirim foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageContentType {
    Text,
    Location,
    Photo,
}

pub mod content_type_name {
    pub const TEXT: &str = "text";
    pub const LOCATION: &str = "location";
    pub const PHOTO: &str = "photo";
}

impl MessageContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageContentType::Text => content_type_name::TEXT,
            MessageContentType::Location => content_type_name::LOCATION,
            MessageContentType::Photo => content_type_name::PHOTO,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            content_type_name::TEXT => Some(MessageContentType::Text),
            content_type_name::LOCATION => Some(MessageContentType::Location),
            content_type_name::PHOTO => Some(MessageContentType::Photo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content_type: MessageContentType,
    /// Hanya terisi bila `content_type == Text`.
    pub content: Option<String>,
    /// Hanya terisi bila `content_type == Location`.
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// Hanya terisi bila `content_type == Photo`.
    pub photo_object_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Input pembuatan satu pesan — tepat satu varian field yang relevan terisi sesuai
/// `content_type` (divalidasi di `ChatService::send_message`, bukan di sini).
#[derive(Debug, Clone)]
pub struct NewMessage {
    pub content_type: MessageContentType,
    pub content: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub photo_object_key: Option<String>,
}
