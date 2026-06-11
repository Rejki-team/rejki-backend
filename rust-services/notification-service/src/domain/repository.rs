use uuid::Uuid;

use super::entity::Notification;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait NotificationRepository: Send + Sync {
    async fn save(
        &self,
        recipient_id: Uuid,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<Notification, anyhow::Error>;

    async fn list_for_user(
        &self,
        recipient_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Notification>, anyhow::Error>;

    async fn mark_read(&self, id: Uuid, recipient_id: Uuid) -> Result<(), anyhow::Error>;
}
