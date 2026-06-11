use uuid::Uuid;

use super::entity::{Conversation, Message};

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait ChatRepository: Send + Sync {
    async fn find_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Conversation, anyhow::Error>;

    async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error>;

    async fn save_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<Message, anyhow::Error>;

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        before_id: Option<Uuid>,
    ) -> Result<Vec<Message>, anyhow::Error>;
}
