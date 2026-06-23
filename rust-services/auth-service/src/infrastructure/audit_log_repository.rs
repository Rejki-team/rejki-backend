use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::audit_log::{AuditContext, AuditEvent, AuditLogRepository};

macro_rules! warn_slow {
    ($start:expr, $op:expr) => {
        let elapsed = $start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::warn!(op = $op, elapsed_ms = elapsed.as_millis(), "slow DB query");
        }
    };
}

/// Repository PostgreSQL untuk tabel `auth.audit_log`.
/// INSERT-only — tidak ada method update atau delete.
pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    async fn log(
        &self,
        user_id: Option<Uuid>,
        event: AuditEvent,
        context: AuditContext,
    ) -> Result<(), anyhow::Error> {
        let t = Instant::now();
        let event_str = event.as_str();
        let metadata = event.metadata();

        sqlx::query(
            "INSERT INTO auth.audit_log (id, user_id, event, ip_address, user_agent, request_id, metadata)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(event_str)
        .bind(context.ip_address.map(|a| a.to_string()))
        .bind(&context.user_agent)
        .bind(&context.request_id)
        .bind(&metadata)
        .execute(&self.pool)
        .await?;

        warn_slow!(t, "auth.audit_log_insert");
        Ok(())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Trait AuditLogRepository bisa diimplementasi oleh mock untuk unit test.
    struct MockAuditLogRepository {
        calls: std::sync::Mutex<Vec<(Option<Uuid>, String, Option<serde_json::Value>)>>,
    }

    impl MockAuditLogRepository {
        fn new() -> Self {
            Self {
                calls: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl AuditLogRepository for MockAuditLogRepository {
        async fn log(
            &self,
            user_id: Option<Uuid>,
            event: AuditEvent,
            _context: AuditContext,
        ) -> Result<(), anyhow::Error> {
            let mut calls = self.calls.lock().unwrap();
            calls.push((user_id, event.as_str().to_owned(), event.metadata()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn mock_audit_log_repository_logs_login_event() {
        let repo = MockAuditLogRepository::new();
        let user_id = Uuid::now_v7();
        let ctx = AuditContext {
            ip_address: None,
            user_agent: None,
            request_id: None,
        };

        repo.log(Some(user_id), AuditEvent::LoginSuccess, ctx)
            .await
            .unwrap();

        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, Some(user_id));
        assert_eq!(calls[0].1, "login_success");
    }

    #[tokio::test]
    async fn mock_audit_log_repository_logs_failed_event_with_none_user_id() {
        let repo = MockAuditLogRepository::new();
        let ctx = AuditContext {
            ip_address: None,
            user_agent: None,
            request_id: None,
        };

        repo.log(
            None,
            AuditEvent::LoginFailed {
                reason: "wrong password".into(),
            },
            ctx,
        )
        .await
        .unwrap();

        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, None);
        assert_eq!(calls[0].1, "login_failed");
    }
}
