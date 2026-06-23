//! Implementasi in-process dari kontrak `NotificationClient`.
//!
//! Memiliki detail infrastruktur (Redis Stream) di dalam domain notification —
//! konsumen lain (mis. auth-service) cukup memanggil trait tanpa tahu Redis.
//! Saat domain notification kelak diekstrak jadi microservice, implementasi ini
//! diganti HTTP client tanpa mengubah konsumen.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use notification_service_client::{
    DeviceToken, DeviceTokenInput, EmailMessage, NotificationClient, NotificationClientError,
    NotificationPayload,
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

/// Channel yang sudah dibuatkan ConnectionManager — reusable.
struct ManagedChannel {
    client: redis::Client,
    conn: Mutex<Option<ConnectionManager>>,
}

/// Publisher Redis Stream sebagai implementasi NotificationClient.
pub struct NotificationPublisher {
    channel: Arc<ManagedChannel>,
}

impl NotificationPublisher {
    pub fn new(redis_url: &str) -> Result<Self, anyhow::Error> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            channel: Arc::new(ManagedChannel {
                client,
                conn: Mutex::new(None),
            }),
        })
    }

    /// Dapatkan atau buat ConnectionManager (lazy init).
    async fn conn(&self) -> Result<ConnectionManager, anyhow::Error> {
        let mut guard = self.channel.conn.lock().await;
        if guard.is_none() {
            *guard = Some(self.channel.client.get_connection_manager().await?);
        }
        Ok(guard.as_ref().unwrap().clone())
    }

    async fn xadd(&self, payload: String) -> Result<(), NotificationClientError> {
        let mut conn = self
            .conn()
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
        // Concurrent batch processing via tokio::spawn
        let mut handles = Vec::with_capacity(recipient_ids.len());
        for id in recipient_ids {
            let payload = payload.clone();
            let this = self.channel.clone();
            let publisher = NotificationPublisher { channel: this };
            handles.push(tokio::spawn(
                async move { publisher.send(id, payload).await },
            ));
        }
        for handle in handles {
            handle
                .await
                .map_err(|_| NotificationClientError::Unavailable)?
                .map_err(|_| NotificationClientError::Unavailable)?;
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

    async fn register_device_token(
        &self,
        _user_id: Uuid,
        _input: DeviceTokenInput,
    ) -> Result<DeviceToken, NotificationClientError> {
        Err(NotificationClientError::Unavailable)
    }

    async fn unregister_device_token(
        &self,
        _user_id: Uuid,
        _token: &str,
    ) -> Result<(), NotificationClientError> {
        Err(NotificationClientError::Unavailable)
    }
}
