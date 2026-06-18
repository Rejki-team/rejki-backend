use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    DeviceTokenResponse, NotificationResponse, RegisterDeviceTokenInput, SendNotificationInput,
};
use crate::domain::repository::NotificationRepository;
use crate::infrastructure::{NotificationEvent, RedisPublisher};
use common_rate_limit::RateLimiter;

pub struct NotificationService<R: NotificationRepository> {
    repo: Arc<R>,
    publisher: Option<Arc<RedisPublisher>>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
}

impl<R: NotificationRepository> NotificationService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            publisher: None,
            rate_limiter: None,
        }
    }

    pub fn with_publisher(repo: Arc<R>, publisher: Arc<RedisPublisher>) -> Self {
        Self {
            repo,
            publisher: Some(publisher),
            rate_limiter: None,
        }
    }

    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    pub async fn send(&self, input: SendNotificationInput) -> Result<(), anyhow::Error> {
        // Rate limit: 20 req/menit per recipient
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("notif:send", &input.recipient_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
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

    // ── Device token ──────────────────────────────────────────────────────

    pub async fn register_device_token(
        &self,
        input: RegisterDeviceTokenInput,
        user_id: Uuid,
    ) -> Result<DeviceTokenResponse, anyhow::Error> {
        // Validate platform
        let platform = match input.platform.as_str() {
            "android" | "ios" | "web" => input.platform.as_str(),
            other => {
                anyhow::bail!("platform tidak dikenal: {other} (gunakan android, ios, atau web)")
            }
        };
        let token = self
            .repo
            .register_device_token(user_id, &input.token, platform)
            .await?;
        Ok(DeviceTokenResponse {
            id: token.id,
            user_id: token.user_id,
            token: token.token,
            platform: token.platform,
            created_at: token.created_at,
            updated_at: token.updated_at,
        })
    }

    pub async fn delete_device_token(
        &self,
        token_str: &str,
        user_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        self.repo.delete_device_token(token_str, user_id).await
    }

    pub async fn list_device_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<DeviceTokenResponse>, anyhow::Error> {
        let tokens = self.repo.list_device_tokens_for_user(user_id).await?;
        Ok(tokens
            .into_iter()
            .map(|t| DeviceTokenResponse {
                id: t.id,
                user_id: t.user_id,
                token: t.token,
                platform: t.platform,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect())
    }
}
