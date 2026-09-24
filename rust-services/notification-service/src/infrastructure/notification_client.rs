//! Implementasi in-process dari kontrak `NotificationClient`.
//!
//! Memiliki detail infrastruktur (Redis Stream) di dalam domain notification —
//! konsumen lain (mis. auth-service) cukup memanggil trait tanpa tahu Redis.
//! Saat domain notification kelak diekstrak jadi microservice, implementasi ini
//! diganti HTTP client tanpa mengubah konsumen.

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::repository::NotificationRepository;
use crate::infrastructure::PgNotificationRepository;
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
    /// Token FCM aktif milik `recipient_id`, di-resolve SEKARANG (saat publish),
    /// bukan di-lookup Bun consumer — Bun jadi stateless untuk Postgres (Opsi C).
    tokens: Vec<String>,
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
///
/// Menyimpan `repo` untuk resolve token FCM SAAT publish (bukan di-lookup oleh
/// Bun consumer) — Opsi C: bun-notification-service jadi stateless untuk Postgres.
pub struct NotificationPublisher {
    channel: Arc<ManagedChannel>,
    repo: Arc<PgNotificationRepository>,
}

impl NotificationPublisher {
    pub fn new(redis_url: &str, pool: PgPool) -> Result<Self, anyhow::Error> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            channel: Arc::new(ManagedChannel {
                client,
                conn: Mutex::new(None),
            }),
            repo: Arc::new(PgNotificationRepository::new(pool)),
        })
    }

    /// Resolve token FCM aktif milik `recipient_id`. Gagal query → treat sebagai
    /// tanpa token (Bun akan skip push, sama seperti perilaku lama saat user
    /// tidak punya token — degradasi anggun, bukan gagal total publish).
    async fn resolve_tokens(&self, recipient_id: Uuid) -> Vec<String> {
        match self.repo.list_device_tokens_for_user(recipient_id).await {
            Ok(tokens) => tokens.into_iter().map(|t| t.token).collect(),
            Err(e) => {
                tracing::warn!(error = ?e, %recipient_id, "gagal resolve FCM token saat publish — publish tanpa token");
                Vec::new()
            }
        }
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
        let tokens = self.resolve_tokens(recipient_id).await;
        let event = PushEvent {
            event_id: new_event_id(),
            recipient_id,
            title: payload.title,
            body: payload.body,
            data: payload.data,
            tokens,
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
            let channel = self.channel.clone();
            let repo = self.repo.clone();
            let publisher = NotificationPublisher { channel, repo };
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
