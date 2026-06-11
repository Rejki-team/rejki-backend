use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::Notification;
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
}
