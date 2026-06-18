use std::sync::Arc;

use crate::application::service::UserService;
use crate::infrastructure::PgUserRepository;
use user_service_client::{UserClient, UserClientError, UserSummary};

/// Implementasi UserClient untuk mode in-process (Modular Monolith).
/// Memanggil UserService secara langsung tanpa HTTP/networking.
#[derive(Clone)]
pub struct UserInProcessClient {
    svc: Arc<UserService<PgUserRepository>>,
}

impl UserInProcessClient {
    pub fn new(svc: Arc<UserService<PgUserRepository>>) -> Self {
        Self { svc }
    }
}

#[async_trait::async_trait]
impl UserClient for UserInProcessClient {
    async fn get_user_summary(&self, user_id: uuid::Uuid) -> Result<UserSummary, UserClientError> {
        let profile = self
            .svc
            .get_profile(user_id)
            .await
            .map_err(|_| UserClientError::NotFound)?;
        Ok(UserSummary {
            id: profile.id,
            username: profile.username,
            avatar: profile.avatar,
        })
    }

    async fn user_exists(&self, user_id: uuid::Uuid) -> Result<bool, UserClientError> {
        // `get_profile` returns Ok=exists or Err=not found.
        Ok(self.svc.get_profile(user_id).await.is_ok())
    }

    async fn purge_kyc_documents(&self, user_id: uuid::Uuid) -> Result<(), UserClientError> {
        self.svc
            .purge_kyc_by_user_id(user_id)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }
}
