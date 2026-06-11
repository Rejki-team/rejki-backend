use uuid::Uuid;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait ChatClient: Send + Sync {
    async fn get_conversation_id(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Uuid, ChatClientError>;
    async fn conversation_exists(&self, conversation_id: Uuid) -> Result<bool, ChatClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChatClientError {
    #[error("conversation not found")]
    NotFound,
    #[error("chat service unavailable")]
    Unavailable,
}
