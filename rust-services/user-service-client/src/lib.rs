use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait UserClient: Send + Sync {
    async fn get_user_summary(&self, user_id: Uuid) -> Result<UserSummary, UserClientError>;
    async fn user_exists(&self, user_id: Uuid) -> Result<bool, UserClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum UserClientError {
    #[error("user not found")]
    NotFound,
    #[error("user service unavailable")]
    Unavailable,
}
