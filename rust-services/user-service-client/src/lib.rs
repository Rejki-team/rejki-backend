use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
}

/// Trait client untuk user-service — dipakai in-process & via HTTP client.
/// `#[async_trait]` diperlukan agar dyn-compatible (auth-service me-wrap sebagai
/// `Arc<dyn UserClient>`).
#[async_trait::async_trait]
pub trait UserClient: Send + Sync {
    async fn get_user_summary(&self, user_id: Uuid) -> Result<UserSummary, UserClientError>;
    async fn user_exists(&self, user_id: Uuid) -> Result<bool, UserClientError>;
    /// Musnahkan dokumen KYC milik pengguna (KTP + Selfie) dari storage +
    /// kosongkan referensi. Dipicu saat suspend permanen (extend-user-suspension-bulk-purge D4).
    /// Idempoten: aman dipanggil ulang.
    async fn purge_kyc_documents(&self, user_id: Uuid) -> Result<(), UserClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum UserClientError {
    #[error("user not found")]
    NotFound,
    #[error("user service unavailable")]
    Unavailable,
}
