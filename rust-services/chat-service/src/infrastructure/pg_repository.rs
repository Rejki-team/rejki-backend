use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::entity::{Conversation, Message, MessageContentType, NewMessage};
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

// Justifikasi #[allow]: setiap parameter memetakan 1:1 ke satu kolom `chat.messages`
// (bukan "banyak argumen tak terkait" — God Function) — hasil query `sqlx::query!`
// anonim tidak bisa di-`impl From<...>` (tipe record generated-nya tidak bisa dinamai
// di luar makro), jadi membungkus jadi struct hanya memindahkan masalah yang sama.
#[allow(clippy::too_many_arguments)]
fn row_to_message(
    id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    content_type: String,
    content: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    photo_object_key: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Message {
    Message {
        id,
        conversation_id,
        sender_id,
        content_type: MessageContentType::parse(&content_type).unwrap_or(MessageContentType::Text),
        content,
        lat,
        lng,
        photo_object_key,
        created_at,
    }
}

impl ChatRepository for PgChatRepository {
    async fn find_or_create_conversation(
        &self,
        user_a: Uuid,
        user_b: Uuid,
        related_ad_type: Option<&str>,
        related_ad_id: Option<Uuid>,
    ) -> Result<(Conversation, bool), anyhow::Error> {
        let (lo, hi) = if user_a < user_b {
            (user_a, user_b)
        } else {
            (user_b, user_a)
        };
        let t = Instant::now();
        let row = sqlx::query!(
            r#"INSERT INTO chat.conversations (id, user_a, user_b, related_ad_type, related_ad_id)
               VALUES (gen_random_uuid(), $1, $2, $3, $4)
               ON CONFLICT (user_a, user_b) DO UPDATE SET
                   related_ad_type = COALESCE($3, chat.conversations.related_ad_type),
                   related_ad_id = COALESCE($4, chat.conversations.related_ad_id)
               RETURNING id, user_a, user_b, created_at, ended_at, related_ad_type, related_ad_id,
                         user_a_last_read_message_id, user_b_last_read_message_id,
                         (xmax = 0) AS is_new"#,
            lo,
            hi,
            related_ad_type,
            related_ad_id
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.find_or_create_conversation");
        Ok((
            Conversation {
                id: row.id,
                user_a: row.user_a,
                user_b: row.user_b,
                created_at: row.created_at,
                ended_at: row.ended_at,
                related_ad_type: row.related_ad_type,
                related_ad_id: row.related_ad_id,
                user_a_last_read_message_id: row.user_a_last_read_message_id,
                user_b_last_read_message_id: row.user_b_last_read_message_id,
            },
            row.is_new.unwrap_or(false),
        ))
    }

    async fn find_conversation_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Conversation>, anyhow::Error> {
        let t = Instant::now();
        let row = sqlx::query!(
            r#"SELECT id, user_a, user_b, created_at, ended_at, related_ad_type, related_ad_id,
                      user_a_last_read_message_id, user_b_last_read_message_id
               FROM chat.conversations WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "chat.find_conversation_by_id");
        Ok(row.map(|row| Conversation {
            id: row.id,
            user_a: row.user_a,
            user_b: row.user_b,
            created_at: row.created_at,
            ended_at: row.ended_at,
            related_ad_type: row.related_ad_type,
            related_ad_id: row.related_ad_id,
            user_a_last_read_message_id: row.user_a_last_read_message_id,
            user_b_last_read_message_id: row.user_b_last_read_message_id,
        }))
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

    async fn is_participant(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                   SELECT 1 FROM chat.conversations
                   WHERE id = $1 AND (user_a = $2 OR user_b = $2)
               )"#,
            conversation_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.is_participant");
        Ok(exists.unwrap_or(false))
    }

    async fn save_message(
        &self,
        conversation_id: Uuid,
        sender_id: Uuid,
        input: &NewMessage,
    ) -> Result<Message, anyhow::Error> {
        let t = Instant::now();
        let content_type = input.content_type.as_str();
        let row = sqlx::query!(
            r#"INSERT INTO chat.messages
                   (id, conversation_id, sender_id, content_type, content, lat, lng, photo_object_key)
               VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7)
               RETURNING id, conversation_id, sender_id, content_type, content, lat, lng,
                         photo_object_key, created_at"#,
            conversation_id,
            sender_id,
            content_type,
            input.content,
            input.lat,
            input.lng,
            input.photo_object_key
        )
        .fetch_one(&self.pool)
        .await?;
        warn_slow!(t, "chat.save_message");
        Ok(row_to_message(
            row.id,
            row.conversation_id,
            row.sender_id,
            row.content_type,
            row.content,
            row.lat,
            row.lng,
            row.photo_object_key,
            row.created_at,
        ))
    }

    async fn list_messages(
        &self,
        conversation_id: Uuid,
        limit: i64,
        before_id: Option<Uuid>,
        after_id: Option<Uuid>,
    ) -> Result<Vec<Message>, anyhow::Error> {
        let t = Instant::now();
        let rows = if let Some(bid) = before_id {
            sqlx::query!(
                r#"SELECT id, conversation_id, sender_id, content_type, content, lat, lng,
                          photo_object_key, created_at
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
            .map(|r| {
                row_to_message(
                    r.id,
                    r.conversation_id,
                    r.sender_id,
                    r.content_type,
                    r.content,
                    r.lat,
                    r.lng,
                    r.photo_object_key,
                    r.created_at,
                )
            })
            .collect()
        } else if let Some(aid) = after_id {
            sqlx::query!(
                r#"SELECT id, conversation_id, sender_id, content_type, content, lat, lng,
                          photo_object_key, created_at
                   FROM chat.messages
                   WHERE conversation_id = $1
                     AND created_at > (SELECT created_at FROM chat.messages WHERE id = $2)
                   ORDER BY created_at ASC LIMIT $3"#,
                conversation_id,
                aid,
                limit
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| {
                row_to_message(
                    r.id,
                    r.conversation_id,
                    r.sender_id,
                    r.content_type,
                    r.content,
                    r.lat,
                    r.lng,
                    r.photo_object_key,
                    r.created_at,
                )
            })
            .collect()
        } else {
            sqlx::query!(
                r#"SELECT id, conversation_id, sender_id, content_type, content, lat, lng,
                          photo_object_key, created_at
                   FROM chat.messages
                   WHERE conversation_id = $1
                   ORDER BY created_at DESC LIMIT $2"#,
                conversation_id,
                limit
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|r| {
                row_to_message(
                    r.id,
                    r.conversation_id,
                    r.sender_id,
                    r.content_type,
                    r.content,
                    r.lat,
                    r.lng,
                    r.photo_object_key,
                    r.created_at,
                )
            })
            .collect()
        };
        warn_slow!(t, "chat.list_messages");
        Ok(rows)
    }

    async fn end_conversation(
        &self,
        id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Conversation>, anyhow::Error> {
        let t = Instant::now();
        // Ownership (membership) check di query — IDOR→404, bukan SELECT lalu compare.
        // Idempoten: sudah berakhir sebelumnya → tetap sukses (COALESCE), bukan error,
        // supaya tombol "Akhiri Percakapan" aman ditekan dobel.
        let row = sqlx::query!(
            r#"UPDATE chat.conversations
               SET ended_at = COALESCE(ended_at, now())
               WHERE id = $1 AND (user_a = $2 OR user_b = $2)
               RETURNING id, user_a, user_b, created_at, ended_at, related_ad_type, related_ad_id,
                         user_a_last_read_message_id, user_b_last_read_message_id"#,
            id,
            actor_id
        )
        .fetch_optional(&self.pool)
        .await?;
        warn_slow!(t, "chat.end_conversation");
        Ok(row.map(|row| Conversation {
            id: row.id,
            user_a: row.user_a,
            user_b: row.user_b,
            created_at: row.created_at,
            ended_at: row.ended_at,
            related_ad_type: row.related_ad_type,
            related_ad_id: row.related_ad_id,
            user_a_last_read_message_id: row.user_a_last_read_message_id,
            user_b_last_read_message_id: row.user_b_last_read_message_id,
        }))
    }

    async fn auto_end_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        let result = sqlx::query!(
            "UPDATE chat.conversations SET ended_at = now() WHERE id = $1 AND ended_at IS NULL",
            id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "chat.auto_end_conversation");
        Ok(result.rows_affected() > 0)
    }

    async fn find_active_conversations_by_ad(
        &self,
        ad_type: &str,
        ad_id: Uuid,
    ) -> Result<Vec<Conversation>, anyhow::Error> {
        let t = Instant::now();
        let rows = sqlx::query!(
            r#"SELECT id, user_a, user_b, created_at, ended_at, related_ad_type, related_ad_id,
                      user_a_last_read_message_id, user_b_last_read_message_id
               FROM chat.conversations
               WHERE related_ad_type = $1 AND related_ad_id = $2 AND ended_at IS NULL"#,
            ad_type,
            ad_id
        )
        .fetch_all(&self.pool)
        .await?;
        warn_slow!(t, "chat.find_active_conversations_by_ad");
        Ok(rows
            .into_iter()
            .map(|row| Conversation {
                id: row.id,
                user_a: row.user_a,
                user_b: row.user_b,
                created_at: row.created_at,
                ended_at: row.ended_at,
                related_ad_type: row.related_ad_type,
                related_ad_id: row.related_ad_id,
                user_a_last_read_message_id: row.user_a_last_read_message_id,
                user_b_last_read_message_id: row.user_b_last_read_message_id,
            })
            .collect())
    }

    async fn purge_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        // FK `fk_messages_conversation ... ON DELETE CASCADE` — messages ikut terhapus.
        let result = sqlx::query!("DELETE FROM chat.conversations WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        warn_slow!(t, "chat.purge_conversation");
        Ok(result.rows_affected() > 0)
    }

    async fn list_conversations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Conversation, Option<Message>)>, anyhow::Error> {
        let t = Instant::now();
        // LATERAL join "pesan terakhir per conversation" — satu query, bukan N+1
        // (Hazard #5) — memanfaatkan index existing `idx_messages_conv_created`.
        let rows = sqlx::query!(
            r#"SELECT c.id, c.user_a, c.user_b, c.created_at, c.ended_at,
                      c.related_ad_type, c.related_ad_id,
                      c.user_a_last_read_message_id, c.user_b_last_read_message_id,
                      m.id AS "msg_id?", m.sender_id AS "msg_sender_id?",
                      m.content_type AS "msg_content_type?", m.content AS "msg_content?",
                      m.lat AS "msg_lat?", m.lng AS "msg_lng?",
                      m.photo_object_key AS "msg_photo_object_key?",
                      m.created_at AS "msg_created_at?"
               FROM chat.conversations c
               LEFT JOIN LATERAL (
                   SELECT * FROM chat.messages
                   WHERE conversation_id = c.id
                   ORDER BY created_at DESC LIMIT 1
               ) m ON true
               WHERE c.user_a = $1 OR c.user_b = $1
               ORDER BY COALESCE(m.created_at, c.created_at) DESC"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        warn_slow!(t, "chat.list_conversations_for_user");
        Ok(rows
            .into_iter()
            .map(|row| {
                let conv = Conversation {
                    id: row.id,
                    user_a: row.user_a,
                    user_b: row.user_b,
                    created_at: row.created_at,
                    ended_at: row.ended_at,
                    related_ad_type: row.related_ad_type,
                    related_ad_id: row.related_ad_id,
                    user_a_last_read_message_id: row.user_a_last_read_message_id,
                    user_b_last_read_message_id: row.user_b_last_read_message_id,
                };
                let last_message = row.msg_id.map(|msg_id| {
                    row_to_message(
                        msg_id,
                        conv.id,
                        row.msg_sender_id.unwrap(),
                        row.msg_content_type.unwrap(),
                        row.msg_content,
                        row.msg_lat,
                        row.msg_lng,
                        row.msg_photo_object_key,
                        row.msg_created_at.unwrap(),
                    )
                });
                (conv, last_message)
            })
            .collect())
    }

    async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<bool, anyhow::Error> {
        let t = Instant::now();
        // Satu statement atomik — ownership check di WHERE (IDOR-safe), tandai terbaca
        // sampai pesan terakhir milik sisi (`user_a`/`user_b`) yang sesuai `user_id`.
        let result = sqlx::query!(
            r#"UPDATE chat.conversations SET
                   user_a_last_read_message_id = CASE WHEN user_a = $2
                       THEN (SELECT id FROM chat.messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT 1)
                       ELSE user_a_last_read_message_id END,
                   user_b_last_read_message_id = CASE WHEN user_b = $2
                       THEN (SELECT id FROM chat.messages WHERE conversation_id = $1 ORDER BY created_at DESC LIMIT 1)
                       ELSE user_b_last_read_message_id END
               WHERE id = $1 AND (user_a = $2 OR user_b = $2)"#,
            id,
            user_id
        )
        .execute(&self.pool)
        .await?;
        warn_slow!(t, "chat.mark_read");
        Ok(result.rows_affected() > 0)
    }
}
