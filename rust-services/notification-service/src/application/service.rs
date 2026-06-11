use std::sync::Arc;
use uuid::Uuid;

use super::dto::{NotificationResponse, SendNotificationInput};
use crate::domain::repository::NotificationRepository;
use crate::infrastructure::{NotificationEvent, RedisPublisher};

pub struct NotificationService<R: NotificationRepository> {
    repo: Arc<R>,
    publisher: Option<Arc<RedisPublisher>>,
}

impl<R: NotificationRepository> NotificationService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            publisher: None,
        }
    }

    pub fn with_publisher(repo: Arc<R>, publisher: Arc<RedisPublisher>) -> Self {
        Self {
            repo,
            publisher: Some(publisher),
        }
    }

    pub async fn send(&self, input: SendNotificationInput) -> Result<(), anyhow::Error> {
        let notif = self
            .repo
            .save(
                input.recipient_id,
                &input.title,
                &input.body,
                input.data.clone(),
            )
            .await?;

        if let Some(pub_) = &self.publisher {
            let event = NotificationEvent {
                event_id: notif.id.to_string(),
                recipient_id: input.recipient_id,
                title: input.title.clone(),
                body: input.body.clone(),
                data: input.data,
            };
            if let Err(e) = pub_.publish(&event).await {
                tracing::warn!(error = ?e, "failed to publish notification to Redis stream");
            }
        }

        tracing::info!(recipient_id = %input.recipient_id, "notification sent");
        Ok(())
    }

    pub async fn send_bulk(
        &self,
        recipient_ids: Vec<Uuid>,
        title: &str,
        body: &str,
    ) -> Result<(), anyhow::Error> {
        for id in recipient_ids {
            self.repo.save(id, title, body, None).await?;
        }
        Ok(())
    }

    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<NotificationResponse>, anyhow::Error> {
        let notifs = self
            .repo
            .list_for_user(user_id, limit.unwrap_or(30))
            .await?;
        Ok(notifs
            .into_iter()
            .map(|n| NotificationResponse {
                id: n.id,
                recipient_id: n.recipient_id,
                title: n.title,
                body: n.body,
                is_read: n.is_read,
                created_at: n.created_at,
            })
            .collect())
    }

    pub async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<(), anyhow::Error> {
        self.repo.mark_read(id, user_id).await
    }
}
