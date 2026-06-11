//! Implementasi in-process dari kontrak `NotificationClient`.
//!
//! Memiliki detail infrastruktur (Redis Stream) di dalam domain notification —
//! konsumen lain (mis. auth-service) cukup memanggil trait tanpa tahu Redis.
//! Saat domain notification kelak diekstrak jadi microservice, implementasi ini
//! diganti HTTP client tanpa mengubah konsumen.

use redis::AsyncCommands;
use serde::Serialize;
use uuid::Uuid;

use notification_service_client::{
    EmailMessage, NotificationClient, NotificationClientError, NotificationPayload,
};

const STREAM: &str = "notifications_stream";

#[derive(Debug, Serialize)]
struct PushEvent {
    event_id: String,
    recipient_id: Uuid,
    title: String,
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct EmailEvent {
    event_id: String,
    channel: &'static str, // "email"
    to: String,
    subject: String,
    body: String,
}

/// Publisher Redis Stream sebagai implementasi NotificationClient.
pub struct NotificationPublisher {
    client: redis::Client,
}

impl NotificationPublisher {
    pub fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        Ok(Self {
            client: redis::Client::open(redis_url)?,
        })
    }

    async fn xadd(&self, payload: String) -> Result<(), NotificationClientError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| NotificationClientError::Unavailable)?;
        let _: String = conn
            .xadd_maxlen(
                STREAM,
                redis::streams::StreamMaxlen::Approx(10_000),
                "*",
                &[("payload", payload)],
            )
            .await
            .map_err(|_| NotificationClientError::Unavailable)?;
        Ok(())
    }
}

fn new_event_id() -> String {
    let ts = Uuid::new_v7(uuid::Timestamp::now(uuid::timestamp::context::NoContext));
    ts.to_string()
}

#[async_trait::async_trait]
impl NotificationClient for NotificationPublisher {
    async fn send(
        &self,
        recipient_id: Uuid,
        payload: NotificationPayload,
    ) -> Result<(), NotificationClientError> {
        let event = PushEvent {
            event_id: new_event_id(),
            recipient_id,
            title: payload.title,
            body: payload.body,
            data: payload.data,
        };
        let s = serde_json::to_string(&event).map_err(|_| NotificationClientError::Unavailable)?;
        self.xadd(s).await
    }

    async fn send_bulk(
        &self,
        recipient_ids: Vec<Uuid>,
        payload: NotificationPayload,
    ) -> Result<(), NotificationClientError> {
        for id in recipient_ids {
            self.send(id, payload.clone()).await?;
        }
        Ok(())
    }

    async fn send_email(&self, message: EmailMessage) -> Result<(), NotificationClientError> {
        let event = EmailEvent {
            event_id: new_event_id(),
            channel: "email",
            to: message.to,
            subject: message.subject,
            body: message.body,
        };
        let s = serde_json::to_string(&event).map_err(|_| NotificationClientError::Unavailable)?;
        self.xadd(s).await
    }
}
