use uuid::Uuid;

use super::entity::{AccountStatus, Role};

/// Abstraction for JWT token issuance.
/// Application layer depends on this trait (DIP), not on concrete JwtService.
pub trait TokenIssuer: Send + Sync {
    /// Issue a signed access token (RS256 JWT) for the given user.
    fn issue_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        status: AccountStatus,
        role: Role,
    ) -> anyhow::Result<String>;
}

/// Abstraction for OTP verification — validates JWT tokens.
/// Used by AuthInProcessClient; not directly by AuthService.
pub trait TokenValidator: Send + Sync {
    fn validate_token(
        &self,
        token: &str,
    ) -> Result<auth_service_client::AuthClaims, auth_service_client::AuthClientError>;
}
