use std::sync::Arc;

use uuid::Uuid;

use chat_service_client::{ChatClient, ChatClientError};

use crate::application::service::ChatService;
use crate::infrastructure::pg_repository::PgChatRepository;

/// Implementasi `ChatClient` untuk mode in-process (Modular Monolith). Dipakai
/// `iklan-pekerjaan-service`/`iklan-barang-bekas-service` untuk menjadwalkan
/// auto-end percakapan setelah "proses pada iklan terkait selesai" (F-19, P4.3).
pub struct ChatInProcessClient {
    svc: Arc<ChatService<PgChatRepository>>,
}

impl ChatInProcessClient {
    pub fn new(svc: Arc<ChatService<PgChatRepository>>) -> Self {
        Self { svc }
    }
}

#[async_trait::async_trait]
impl ChatClient for ChatInProcessClient {
    async fn get_conversation_id(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Uuid, ChatClientError> {
        self.svc
            .get_or_create_conversation(user_a, user_b, None, None)
            .await
            .map(|resp| resp.id)
            .map_err(|_| ChatClientError::Unavailable)
    }

    async fn conversation_exists(&self, conversation_id: Uuid) -> Result<bool, ChatClientError> {
        self.svc
            .conversation_exists(conversation_id)
            .await
            .map_err(|_| ChatClientError::Unavailable)
    }

    async fn schedule_auto_end_for_ad(
        &self,
        ad_type: &str,
        ad_id: Uuid,
    ) -> Result<(), ChatClientError> {
        self.svc
            .schedule_auto_end_for_ad(ad_type, ad_id)
            .await
            .map_err(|_| ChatClientError::Unavailable)
    }
}
