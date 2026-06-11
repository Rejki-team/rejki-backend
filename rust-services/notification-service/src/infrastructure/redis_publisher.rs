use redis::AsyncCommands;
use serde::Serialize;
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
}

impl RedisPublisher {
    pub fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    /// Publish event ke Redis Stream `notifications_stream`.
    /// MAXLEN ~10_000 untuk mencegah stream tumbuh tak terbatas.
    pub async fn publish(&self, event: &NotificationEvent) -> Result<(), anyhow::Error> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
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
