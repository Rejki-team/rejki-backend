use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Event yang dipublish ke Redis Stream untuk dikonsumsi oleh bun-notification-service.
#[derive(Debug, Serialize)]
pub struct NotificationEvent {
    pub event_id: String,
    pub recipient_id: Uuid,
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

pub struct RedisPublisher {
    client: redis::Client,
    conn: Mutex<Option<ConnectionManager>>,
}

impl RedisPublisher {
    pub fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            client,
            conn: Mutex::new(None),
        })
    }

    /// Dapatkan atau buat ConnectionManager (lazy init).
    async fn conn(&self) -> Result<ConnectionManager, anyhow::Error> {
        let mut guard = self.conn.lock().await;
        if guard.is_none() {
            *guard = Some(self.client.get_connection_manager().await?);
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    /// Publish event ke Redis Stream `notifications_stream`.
    /// MAXLEN ~10_000 untuk mencegah stream tumbuh tak terbatas.
    pub async fn publish(&self, event: &NotificationEvent) -> Result<(), anyhow::Error> {
        let mut conn = self.conn().await?;
        let payload = serde_json::to_string(event)?;

        let _: String = conn
            .xadd_maxlen(
                "notifications_stream",
                redis::streams::StreamMaxlen::Approx(10_000),
                "*",
                &[("payload", payload)],
            )
            .await?;

        tracing::debug!(
            event_id   = %event.event_id,
            recipient  = %event.recipient_id,
            "notification event published to Redis stream"
        );
        Ok(())
    }
}
