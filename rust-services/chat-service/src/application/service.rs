use std::sync::Arc;
use uuid::Uuid;

use super::dto::{ConversationResponse, ListMessagesQuery, MessageResponse, SendMessageInput};
use crate::domain::entity::Message;
use crate::domain::repository::ChatRepository;
use common_errors::CursorMeta;
use common_rate_limit::RateLimiter;

pub struct ChatService<R: ChatRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
}

impl<R: ChatRepository> ChatService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    pub async fn get_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<ConversationResponse, anyhow::Error> {
        let conv = self
            .repo
            .find_or_create_conversation(user_a, user_b)
            .await?;
        Ok(ConversationResponse {
            id: conv.id,
            user_a: conv.user_a,
            user_b: conv.user_b,
        })
    }

    pub async fn send_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        input: SendMessageInput,
    ) -> Result<MessageResponse, anyhow::Error> {
        let exists = self.repo.conversation_exists(conversation_id).await?;
        // Rate limit: 30 req/menit per sender
        if let Some(rl) = &self.rate_limiter {
            if !rl.allow("chat:send_message", &sender_id.to_string()).await {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        if !exists {
            return Err(anyhow::anyhow!("conversation tidak ditemukan"));
        }

        let clean_content = ammonia::clean_text(&input.content);
        let msg = self
            .repo
            .save_message(conversation_id, sender_id, &clean_content)
            .await?;

        Ok(MessageResponse {
            id: msg.id,
            conversation_id: msg.conversation_id,
            sender_id: msg.sender_id,
            content: msg.content,
            created_at: msg.created_at,
        })
    }

    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        query: ListMessagesQuery,
    ) -> Result<(Vec<MessageResponse>, Option<CursorMeta>), anyhow::Error> {
        let limit = query.limit.unwrap_or(50);
        // Fetch satu ekstra untuk menentukan has_more
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

        let response: Vec<MessageResponse> = items
            .into_iter()
            .map(|m| MessageResponse {
                id: m.id,
                conversation_id: m.conversation_id,
                sender_id: m.sender_id,
                content: m.content,
                created_at: m.created_at,
            })
            .collect();

        Ok((response, cursor))
    }
}
