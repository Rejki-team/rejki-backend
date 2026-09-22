use std::sync::Arc;

use crate::application::service::UserService;
use crate::infrastructure::PgUserRepository;
use user_service_client::{
    SensitiveDocFlags, UserClient, UserClientError, UserLocationSummary, UserSummary,
};

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

    async fn get_sensitive_doc_flags(
        &self,
        auth_id: uuid::Uuid,
    ) -> Result<SensitiveDocFlags, UserClientError> {
        self.svc
            .get_sensitive_doc_flags(auth_id)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }

    async fn admin_reveal_nik(
        &self,
        auth_id: uuid::Uuid,
        admin_id: uuid::Uuid,
    ) -> Result<Option<String>, UserClientError> {
        self.svc
            .admin_reveal_nik_by_auth_id(auth_id, admin_id)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }

    async fn admin_get_document_url(
        &self,
        auth_id: uuid::Uuid,
        kind: &str,
        admin_id: uuid::Uuid,
    ) -> Result<Option<String>, UserClientError> {
        self.svc
            .admin_get_document_url_by_auth_id(auth_id, kind, admin_id)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }

    async fn get_location_summaries_by_auth_ids(
        &self,
        auth_ids: &[uuid::Uuid],
    ) -> Result<Vec<UserLocationSummary>, UserClientError> {
        self.svc
            .get_location_summaries(auth_ids)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }

    async fn get_demographic_summaries_by_auth_ids(
        &self,
        auth_ids: &[uuid::Uuid],
    ) -> Result<Vec<user_service_client::UserDemographicSummary>, UserClientError> {
        self.svc
            .get_demographic_summaries(auth_ids)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }

    async fn get_summaries_by_auth_ids(
        &self,
        auth_ids: &[uuid::Uuid],
    ) -> Result<Vec<UserSummary>, UserClientError> {
        self.svc
            .get_summaries(auth_ids)
            .await
            .map_err(|_| UserClientError::Unavailable)
    }
}
