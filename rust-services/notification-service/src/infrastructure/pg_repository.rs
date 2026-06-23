use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{DeviceToken, Notification};
use crate::domain::repository::NotificationRepository;

pub struct PgNotificationRepository {
    pool: PgPool,
}

impl PgNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

macro_rules! warn_slow {
    ($start:expr, $op:expr) => {
        let elapsed = $start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::warn!(op = $op, elapsed_ms = elapsed.as_millis(), "slow DB query");
        }
    };
}

impl NotificationRepository for PgNotificationRepository {
    async fn save(
        &self,
        recipient_id: Uuid,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<Notification, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO notification.notifications (id, recipient_id, title, body, data)
               VALUES (gen_random_uuid(), $1, $2, $3, $4)
               RETURNING id, recipient_id, title, body, data, is_read, created_at"#,
            recipient_id,
            title,
            body,
            data
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "notification.save");

        Ok(Notification {
            id: row.id,
            recipient_id: row.recipient_id,
            title: row.title,
            body: row.body,
            data: row.data,
            is_read: row.is_read,
            created_at: row.created_at,
        })
    }

    async fn list_for_user(
        &self,
        recipient_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Notification>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!(
            r#"SELECT id, recipient_id, title, body, data, is_read, created_at
               FROM notification.notifications
               WHERE recipient_id = $1
               ORDER BY created_at DESC LIMIT $2"#,
            recipient_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        warn_slow!(t, "notification.list_for_user");

        Ok(rows
            .into_iter()
            .map(|r| Notification {
                id: r.id,
                recipient_id: r.recipient_id,
                title: r.title,
                body: r.body,
                data: r.data,
                is_read: r.is_read,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn mark_read(&self, id: Uuid, recipient_id: Uuid) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        sqlx::query!(
            "UPDATE notification.notifications SET is_read = true WHERE id = $1 AND recipient_id = $2",
            id, recipient_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "notification.mark_read");
        Ok(())
    }

    async fn register_device_token(
        &self,
        user_id: Uuid,
        token: &str,
        platform: &str,
    ) -> Result<DeviceToken, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO notification.device_tokens (id, user_id, token, platform)
               VALUES (gen_random_uuid(), $1, $2, $3)
               ON CONFLICT (token) DO UPDATE SET platform = $3, updated_at = now()
               RETURNING id, user_id, token, platform, created_at, updated_at"#,
            user_id,
            token,
            platform,
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "notification.register_device_token");

        Ok(DeviceToken {
            id: row.id,
            user_id: row.user_id,
            token: row.token,
            platform: row.platform,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn delete_device_token(&self, token: &str, user_id: Uuid) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        sqlx::query!(
            "DELETE FROM notification.device_tokens WHERE token = $1 AND user_id = $2",
            token,
            user_id,
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "notification.delete_device_token");
        Ok(())
    }

    async fn list_device_tokens_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<DeviceToken>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!(
            r#"SELECT id, user_id, token, platform, created_at, updated_at
               FROM notification.device_tokens
               WHERE user_id = $1
               ORDER BY created_at DESC"#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;
        warn_slow!(t, "notification.list_device_tokens_for_user");

        Ok(rows
            .into_iter()
            .map(|r| DeviceToken {
                id: r.id,
                user_id: r.user_id,
                token: r.token,
                platform: r.platform,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }
}
