use std::sync::Arc;

use uuid::Uuid;

use auth_service_client::{AccountStatus, AuthClaims, AuthClient, AuthClientError};

use super::jwt::JwtService;
use super::pg_repository::PgAuthRepository;
use crate::domain::repository::AuthRepository;

/// Implementasi AuthClient untuk mode in-process (Modular Monolith).
/// Dipakai oleh rejki-app sebagai Arc<dyn AuthClient> yang di-inject ke middleware
/// dan ke service lain (user-service) untuk membaca/mengubah status akun.
///
/// Mengikat ke PgAuthRepository (Composition Root selalu memakai Postgres). Saat
/// diekstrak jadi microservice, implementasi AuthClient diganti HttpAuthClient.
pub struct AuthInProcessClient {
    jwt: Arc<JwtService>,
    repo: Arc<PgAuthRepository>,
}

impl AuthInProcessClient {
    pub fn new(jwt: Arc<JwtService>, repo: Arc<PgAuthRepository>) -> Self {
        Self { jwt, repo }
    }
}

#[async_trait::async_trait]
impl AuthClient for AuthInProcessClient {
    async fn validate_token(&self, token: &str) -> Result<AuthClaims, AuthClientError> {
        self.jwt.validate_token(token)
    }

    async fn get_account_status(&self, user_id: Uuid) -> Result<AccountStatus, AuthClientError> {
        self.repo
            .get_status(user_id)
            .await
            .map_err(|_| AuthClientError::Unavailable)?
            .ok_or(AuthClientError::NotFound)
    }

    async fn set_account_status(
        &self,
        user_id: Uuid,
        status: AccountStatus,
    ) -> Result<(), AuthClientError> {
        // Validasi transisi pada state machine sebelum menulis.
        let current = self
            .repo
            .get_status(user_id)
            .await
            .map_err(|_| AuthClientError::Unavailable)?
            .ok_or(AuthClientError::NotFound)?;

        if !current.can_transition_to(status) {
            return Err(AuthClientError::InvalidTransition);
        }

        self.repo
            .set_status(user_id, status)
            .await
            .map_err(|_| AuthClientError::Unavailable)
    }
}
