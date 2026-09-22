use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    ConversationListItemResponse, ConversationResponse, ListMessagesQuery, MessageResponse,
    PhotoUploadPermissionResponse, SendMessageInput,
};
use crate::domain::entity::{Conversation, Message, MessageContentType, NewMessage};
use crate::domain::repository::ChatRepository;
use common_errors::CursorMeta;
use common_rate_limit::RateLimiter;
use common_scheduler::{JobEnvelope, SchedulerClient};
use std::collections::HashMap;
use storage_service_client::{FileInfo, StorageClient};
use user_service_client::UserClient;

/// Auto-end 2x24 jam setelah "proses pada iklan terkait selesai" (PRD §5.9/Bab 10).
pub const AUTO_END_DELAY_HOURS: i64 = 48;
/// Retensi riwayat percakapan 60 hari setelah dibuat (PRD §5.9/Bab 10).
pub const RETENTION_DAYS: i64 = 60;

pub mod storage_category {
    pub const CHAT_PHOTO: &str = "chat-photo";
}

pub struct ChatService<R: ChatRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
    scheduler_client: Option<Arc<SchedulerClient>>,
    storage_client: Option<Arc<dyn StorageClient>>,
    /// P4.10 — enrichment nama+avatar lawan bicara di "daftar percakapan".
    user_client: Option<Arc<dyn UserClient>>,
}

impl<R: ChatRepository> ChatService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
            scheduler_client: None,
            storage_client: None,
            user_client: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    pub fn with_scheduler_client(mut self, sc: Arc<SchedulerClient>) -> Self {
        self.scheduler_client = Some(sc);
        self
    }

    pub fn with_storage_client(mut self, sc: Arc<dyn StorageClient>) -> Self {
        self.storage_client = Some(sc);
        self
    }

    pub fn with_user_client(mut self, uc: Arc<dyn UserClient>) -> Self {
        self.user_client = Some(uc);
        self
    }

    /// Jadwalkan satu tugas otomatis (F-32 generik) — fail-open: gagal menjadwalkan
    /// TIDAK PERNAH menggagalkan operasi utama, hanya `warn!` (§4.5 backend I/O aman).
    async fn schedule_job(
        &self,
        job_type: &str,
        payload: serde_json::Value,
        execute_after: chrono::DateTime<chrono::Utc>,
    ) {
        let Some(scheduler) = &self.scheduler_client else {
            return;
        };
        let envelope = JobEnvelope::new(job_type, payload, execute_after);
        if let Err(e) = scheduler.schedule(&envelope).await {
            tracing::warn!(job_type, error = ?e, "gagal menjadwalkan tugas chat — dilewati (fail-open)");
        }
    }

    pub async fn get_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
        related_ad_type: Option<&str>,
        related_ad_id: Option<Uuid>,
    ) -> Result<ConversationResponse, anyhow::Error> {
        let (conv, is_new) = self
            .repo
            .find_or_create_conversation(user_a, user_b, related_ad_type, related_ad_id)
            .await?;

        if is_new {
            // P4.5: retensi 60 hari — dijadwalkan SEKALI saat baris baru dibuat.
            self.schedule_job(
                crate::application::scheduled_jobs::JOB_CHAT_RETENTION_PURGE,
                serde_json::json!({ "conversation_id": conv.id }),
                conv.created_at + chrono::Duration::days(RETENTION_DAYS),
            )
            .await;
        }

        Ok(to_conversation_response(conv))
    }

    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        input: SendMessageInput,
    ) -> Result<MessageResponse, anyhow::Error> {
        // Rate limit: 30 req/menit per sender
        if let Some(rl) = &self.rate_limiter {
            if !rl.allow("chat:send_message", &sender_id.to_string()).await {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        // Membership check di query yang sama dengan exists — IDOR→404, generic
        // "tidak ditemukan" (tidak membedakan "tidak ada" vs "bukan participant").
        if !self.repo.is_participant(conversation_id, sender_id).await? {
            return Err(anyhow::anyhow!("conversation tidak ditemukan"));
        }

        let new_message = match input {
            SendMessageInput::Text { content } => {
                let len = content.trim().chars().count();
                if !(1..=4000).contains(&len) {
                    return Err(anyhow::anyhow!("pesan harus 1-4000 karakter"));
                }
                NewMessage {
                    content_type: MessageContentType::Text,
                    content: Some(ammonia::clean_text(&content)),
                    lat: None,
                    lng: None,
                    photo_object_key: None,
                }
            }
            SendMessageInput::Location { lat, lng } => {
                if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
                    return Err(anyhow::anyhow!("koordinat lokasi tidak valid"));
                }
                NewMessage {
                    content_type: MessageContentType::Location,
                    content: None,
                    lat: Some(lat),
                    lng: Some(lng),
                    photo_object_key: None,
                }
            }
            SendMessageInput::Photo { photo_object_key } => {
                if photo_object_key.trim().is_empty() {
                    return Err(anyhow::anyhow!("photo_object_key wajib diisi"));
                }
                NewMessage {
                    content_type: MessageContentType::Photo,
                    content: None,
                    lat: None,
                    lng: None,
                    photo_object_key: Some(photo_object_key),
                }
            }
        };

        let msg = self
            .repo
            .save_message(conversation_id, sender_id, &new_message)
            .await?;

        Ok(to_message_response(msg))
    }

    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        query: ListMessagesQuery,
    ) -> Result<(Vec<MessageResponse>, Option<CursorMeta>), anyhow::Error> {
        let limit = query.limit.unwrap_or(50);
        let messages = self
            .repo
            .list_messages(conversation_id, limit + 1, query.before_id, query.after_id)
            .await?;

        let has_more = messages.len() > limit as usize;
        let items: Vec<Message> = messages.into_iter().take(limit as usize).collect();

        let next_cursor = if has_more {
            items.last().map(|m| m.id.to_string())
        } else {
            None
        };

        let cursor = if next_cursor.is_some() || has_more {
            Some(CursorMeta {
                next_cursor,
                has_more,
            })
        } else {
            None
        };

        let response: Vec<MessageResponse> = items.into_iter().map(to_message_response).collect();

        Ok((response, cursor))
    }

    /// P4.4 — akhiri percakapan manual. IDOR→404 (bukan participant → error generik).
    pub async fn end_conversation(
        &self,
        conversation_id: Uuid,
        actor_id: Uuid,
    ) -> Result<ConversationResponse, anyhow::Error> {
        self.repo
            .end_conversation(conversation_id, actor_id)
            .await?
            .map(to_conversation_response)
            .ok_or_else(|| anyhow::anyhow!("conversation tidak ditemukan"))
    }

    /// P4.2 — minta izin unggah foto (presigned URL), kategori storage `chat-photo`.
    pub async fn request_photo_upload(
        &self,
        sender_id: Uuid,
        mime: String,
        size_bytes: u64,
    ) -> Result<PhotoUploadPermissionResponse, anyhow::Error> {
        let storage = self
            .storage_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("storage tidak tersedia"))?;
        let perm = storage
            .request_upload(
                storage_category::CHAT_PHOTO,
                sender_id,
                FileInfo { mime, size_bytes },
            )
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(PhotoUploadPermissionResponse {
            presigned_url: perm.presigned_url,
            object_key: perm.object_key,
        })
    }

    /// P4.3 — dipanggil `ChatInProcessClient` (implementasi `ChatClient`) setelah
    /// service lain melaporkan "proses pada iklan terkait selesai". Menjadwalkan
    /// auto-end +2x24 jam untuk SETIAP conversation aktif yang terhubung ke iklan ini.
    pub async fn schedule_auto_end_for_ad(
        &self,
        ad_type: &str,
        ad_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        let conversations = self
            .repo
            .find_active_conversations_by_ad(ad_type, ad_id)
            .await?;
        for conv in conversations {
            self.schedule_job(
                crate::application::scheduled_jobs::JOB_CHAT_AUTO_END,
                serde_json::json!({ "conversation_id": conv.id }),
                chrono::Utc::now() + chrono::Duration::hours(AUTO_END_DELAY_HOURS),
            )
            .await;
        }
        Ok(())
    }

    pub async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.conversation_exists(id).await
    }

    pub async fn is_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        self.repo.is_participant(conversation_id, user_id).await
    }

    /// P4.10 — "Halaman daftar percakapan" (F-18, PRD §5.9). `other_user` diperkaya
    /// via SATU batch call ke `UserClient` (Hazard #5, bukan N+1 per conversation) —
    /// fail-open: `UserClient` tidak terpasang/gagal → seluruh `other_user` `None`,
    /// list tetap dikembalikan (degradasi anggun, §4.5 backend).
    pub async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationListItemResponse>, anyhow::Error> {
        let raw = self.repo.list_conversations_for_user(user_id).await?;

        let other_user_ids: Vec<Uuid> = raw
            .iter()
            .map(|(conv, _)| other_participant(conv, user_id))
            .collect();

        let summaries: HashMap<Uuid, user_service_client::UserSummary> = if let Some(uc) =
            &self.user_client
        {
            match uc.get_summaries_by_auth_ids(&other_user_ids).await {
                Ok(list) => list.into_iter().map(|s| (s.id, s)).collect(),
                Err(e) => {
                    tracing::warn!(error = ?e, "gagal enrich other_user di daftar percakapan — dilewati (fail-open)");
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        Ok(raw
            .into_iter()
            .map(|(conv, last_message)| {
                let other_user_id = other_participant(&conv, user_id);
                let my_last_read = if conv.user_a == user_id {
                    conv.user_a_last_read_message_id
                } else {
                    conv.user_b_last_read_message_id
                };
                let has_unread = last_message
                    .as_ref()
                    .map(|m| m.sender_id != user_id && Some(m.id) != my_last_read)
                    .unwrap_or(false);
                ConversationListItemResponse {
                    id: conv.id,
                    other_user: summaries.get(&other_user_id).cloned(),
                    last_message: last_message.map(to_message_response),
                    has_unread,
                    ended_at: conv.ended_at,
                    related_ad_type: conv.related_ad_type,
                    related_ad_id: conv.related_ad_id,
                }
            })
            .collect())
    }

    /// P4.11 — tandai `user_id` sudah membaca sampai pesan terakhir. IDOR→404 (ownership
    /// check di repository query, bukan SELECT lalu compare).
    pub async fn mark_read(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        let updated = self.repo.mark_read(conversation_id, user_id).await?;
        if !updated {
            return Err(anyhow::anyhow!("conversation tidak ditemukan"));
        }
        Ok(())
    }
}

fn other_participant(conv: &Conversation, user_id: Uuid) -> Uuid {
    if conv.user_a == user_id {
        conv.user_b
    } else {
        conv.user_a
    }
}

fn to_conversation_response(conv: Conversation) -> ConversationResponse {
    ConversationResponse {
        id: conv.id,
        user_a: conv.user_a,
        user_b: conv.user_b,
        ended_at: conv.ended_at,
        related_ad_type: conv.related_ad_type,
        related_ad_id: conv.related_ad_id,
    }
}

fn to_message_response(m: Message) -> MessageResponse {
    MessageResponse {
        id: m.id,
        conversation_id: m.conversation_id,
        sender_id: m.sender_id,
        content_type: m.content_type.as_str().to_string(),
        content: m.content,
        lat: m.lat,
        lng: m.lng,
        photo_object_key: m.photo_object_key,
        created_at: m.created_at,
    }
}
