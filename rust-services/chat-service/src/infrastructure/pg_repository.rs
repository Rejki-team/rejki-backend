use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{Conversation, Message};
use crate::domain::repository::ChatRepository;

pub struct PgChatRepository {
    pool: PgPool,
}

impl PgChatRepository {
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

impl ChatRepository for PgChatRepository {
    async fn find_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
    ) -> Result<Conversation, anyhow::Error> {
        let (lo, hi) = if user_a < user_b {
            (user_a, user_b)
        } else {
            (user_b, user_a)
        };
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO chat.conversations (id, user_a, user_b)
               VALUES (gen_random_uuid(), $1, $2)
               ON CONFLICT (user_a, user_b) DO UPDATE SET id = chat.conversations.id
               RETURNING id, user_a, user_b, created_at"#,
            lo,
            hi
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.find_or_create_conversation");
        Ok(Conversation {
            id: row.id,
            user_a: row.user_a,
            user_b: row.user_b,
            created_at: row.created_at,
        })
    }

    async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM chat.conversations WHERE id = $1)",
            id
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.conversation_exists");
        Ok(exists.unwrap_or(false))
    }

    async fn save_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        content: &str,
    ) -> Result<Message, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO chat.messages (id, conversation_id, sender_id, content)
               VALUES (gen_random_uuid(), $1, $2, $3)
               RETURNING id, conversation_id, sender_id, content, created_at"#,
            conversation_id,
            sender_id,
            content
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.save_message");
        Ok(Message {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_id: row.sender_id,
            content: row.content,
            created_at: row.created_at,
        })
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        before_id: Option<Uuid>,
    ) -> Result<Vec<Message>, anyhow::Error> {
        let t = Instant::now();
        let rows = if let Some(bid) = before_id {
            sqlx::query!(
                r#"SELECT id, conversation_id, sender_id, content, created_at
                   FROM chat.messages
                   WHERE conversation_id = $1
                     AND created_at < (SELECT created_at FROM chat.messages WHERE id = $2)
                   ORDER BY created_at DESC LIMIT $3"#,
                conversation_id,
                bid,
                limit
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| Message {
                id: r.id,
                conversation_id: r.conversation_id,
                sender_id: r.sender_id,
                content: r.content,
                created_at: r.created_at,
            })
            .collect()
        } else {
            sqlx::query!(
                r#"SELECT id, conversation_id, sender_id, content, created_at
                   FROM chat.messages
                   WHERE conversation_id = $1
                   ORDER BY created_at DESC LIMIT $2"#,
                conversation_id,
                limit
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| Message {
                id: r.id,
                conversation_id: r.conversation_id,
                sender_id: r.sender_id,
                content: r.content,
                created_at: r.created_at,
            })
            .collect()
        };
        warn_slow!(t, "chat.list_messages");
        Ok(rows)
    }
}
