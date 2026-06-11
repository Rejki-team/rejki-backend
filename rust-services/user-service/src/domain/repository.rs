use uuid::Uuid;

use super::entity::{KycSubmission, KycSubmissionStatus, UserProfile};

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    // Profil
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    async fn find_by_auth_id(&self, auth_id: Uuid) -> Result<Option<UserProfile>, anyhow::Error>;
    async fn create(&self, auth_id: Uuid, username: &str) -> Result<UserProfile, anyhow::Error>;
    async fn update(
        &self,
        id: Uuid,
        full_name: Option<&str>,
        avatar: Option<&str>,
        bio: Option<&str>,
        phone: Option<&str>,
    ) -> Result<UserProfile, anyhow::Error>;
    async fn update_avatar(&self, id: Uuid, object_key: &str) -> Result<(), anyhow::Error>;
    /// Perbarui SEMUA field profil (termasuk KYC) — dipakai untuk simpan data diri.
    async fn update_profile(&self, profile: &UserProfile) -> Result<(), anyhow::Error>;

    // KYC submission
    async fn create_submission(&self, profile_id: Uuid) -> Result<KycSubmission, anyhow::Error>;
    async fn get_submission_by_id(&self, submission_id: Uuid) -> Result<Option<KycSubmission>, anyhow::Error>;
    async fn get_latest_submission(&self, profile_id: Uuid) -> Result<Option<KycSubmission>, anyhow::Error>;
    async fn review_submission(
        &self,
        id:          Uuid,
        status:      KycSubmissionStatus,
        reviewed_by: Uuid,
        review_note: Option<&str>,
    ) -> Result<(), anyhow::Error>;
}
