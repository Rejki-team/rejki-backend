use std::sync::Arc;
use uuid::Uuid;

use super::dto::{ConversationResponse, ListMessagesQuery, MessageResponse, SendMessageInput};
use crate::domain::repository::ChatRepository;

pub struct ChatService<R: ChatRepository> {
    repo: Arc<R>,
}

impl<R: ChatRepository> ChatService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
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
    ) -> Result<Vec<MessageResponse>, anyhow::Error> {
        let messages = self
            .repo
            .list_messages(conversation_id, query.limit.unwrap_or(50), query.before_id)
            .await?;

        Ok(messages
            .into_iter()
            .map(|m| MessageResponse {
                id: m.id,
                conversation_id: m.conversation_id,
                sender_id: m.sender_id,
                content: m.content,
                created_at: m.created_at,
            })
            .collect())
    }
}
