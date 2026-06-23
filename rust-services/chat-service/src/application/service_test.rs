#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{ListMessagesQuery, SendMessageInput};
    use crate::application::service::ChatService;
    use crate::domain::entity::{Conversation, Message};
    use crate::domain::repository::ChatRepository;

    // ── MockChatRepository ──────────────────────────────────────────────────────

    struct MockChatRepository {
        conversations: Mutex<Vec<Conversation>>,
        messages: Mutex<Vec<Message>>,
    }

    impl MockChatRepository {
        fn new() -> Self {
            Self {
                conversations: Mutex::new(vec![]),
                messages: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl ChatRepository for MockChatRepository {
        async fn find_or_create_conversation(
            &self,
            user_a: Uuid,
            user_b: Uuid,
        ) -> Result<Conversation, anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            if let Some(existing) = convs.iter().find(|c| {
                (c.user_a == user_a && c.user_b == user_b)
                    || (c.user_a == user_b && c.user_b == user_a)
            }) {
                return Ok(existing.clone());
            }
            let new_conv = Conversation {
                id: Uuid::now_v7(),
                user_a,
                user_b,
                created_at: chrono::Utc::now(),
            };
            convs.push(new_conv.clone());
            Ok(new_conv)
        }

        async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(self
                .conversations
                .lock()
                .unwrap()
                .iter()
                .any(|c| c.id == id))
        }

        async fn save_message(
            &self,
            conversation_id: Uuid,
            sender_id: Uuid,
            content: &str,
        ) -> Result<Message, anyhow::Error> {
            let msg = Message {
                id: Uuid::now_v7(),
                conversation_id,
                sender_id,
                content: content.to_string(),
                created_at: chrono::Utc::now(),
            };
            self.messages.lock().unwrap().push(msg.clone());
            Ok(msg)
        }

        async fn list_messages(
            &self,
            conversation_id: Uuid,
            limit: i64,
            _before_id: Option<Uuid>,
            _after_id: Option<Uuid>,
        ) -> Result<Vec<Message>, anyhow::Error> {
            let msgs = self.messages.lock().unwrap();
            let filtered: Vec<_> = msgs
                .iter()
                .filter(|m| m.conversation_id == conversation_id)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok(filtered)
        }
    }

    fn svc() -> ChatService<MockChatRepository> {
        ChatService::new(std::sync::Arc::new(MockChatRepository::new()))
    }

    // ── get_or_create_conversation ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_or_create_conversation_given_new_pair_when_create_then_returns_conv() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        assert_eq!(conv.user_a, ua);
        assert_eq!(conv.user_b, ub);
        assert!(!conv.id.is_nil());
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_given_existing_pair_when_call_again_then_same_id() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let c1 = s.get_or_create_conversation(ua, ub).await.unwrap();
        let c2 = s.get_or_create_conversation(ua, ub).await.unwrap();
        assert_eq!(c1.id, c2.id);
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_given_reversed_pair_when_call_then_same_conv() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let c1 = s.get_or_create_conversation(ua, ub).await.unwrap();
        let c2 = s.get_or_create_conversation(ub, ua).await.unwrap();
        assert_eq!(c1.id, c2.id);
    }

    // ── send_message ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_send_message_given_valid_conversation_when_send_then_succeeds() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        let msg = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput {
                    content: "Halo!".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(msg.conversation_id, conv.id);
        assert_eq!(msg.sender_id, ua);
        assert_eq!(msg.content, "Halo!");
    }

    #[tokio::test]
    async fn test_send_message_given_nonexistent_conversation_when_send_then_returns_error() {
        let s = svc();
        let result = s
            .send_message(
                Uuid::now_v7(),
                Uuid::now_v7(),
                SendMessageInput {
                    content: "X".into(),
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tidak ditemukan"));
    }

    #[tokio::test]
    async fn test_send_message_given_html_content_when_send_then_strips_html() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        let msg = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput {
                    content: "<script>alert('xss')</script>Hello".into(),
                },
            )
            .await
            .unwrap();
        assert!(!msg.content.contains('<'));
        assert!(msg.content.contains("Hello"));
    }

    // ── list_messages ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_messages_given_messages_when_list_then_returns_all() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        s.send_message(
            conv.id,
            ua,
            SendMessageInput {
                content: "A".into(),
            },
        )
        .await
        .unwrap();
        s.send_message(
            conv.id,
            ub,
            SendMessageInput {
                content: "B".into(),
            },
        )
        .await
        .unwrap();
        let (msgs, cursor) = s
            .list_messages(
                conv.id,
                ListMessagesQuery {
                    limit: Some(10),
                    before_id: None,
                    after_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(msgs.len(), 2);
        assert!(cursor.is_none());
    }

    #[tokio::test]
    async fn test_list_messages_given_more_than_limit_when_list_then_has_more_true() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        for i in 0..5 {
            s.send_message(
                conv.id,
                ua,
                SendMessageInput {
                    content: format!("Msg {i}"),
                },
            )
            .await
            .unwrap();
        }
        let (msgs, cursor) = s
            .list_messages(
                conv.id,
                ListMessagesQuery {
                    limit: Some(3),
                    before_id: None,
                    after_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(msgs.len(), 3);
        assert!(cursor.is_some());
        let c = cursor.unwrap();
        assert!(c.has_more);
        assert!(c.next_cursor.is_some());
    }

    #[tokio::test]
    async fn test_list_messages_given_no_messages_when_list_then_returns_empty() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        let (msgs, cursor) = s
            .list_messages(
                conv.id,
                ListMessagesQuery {
                    limit: Some(10),
                    before_id: None,
                    after_id: None,
                },
            )
            .await
            .unwrap();
        assert!(msgs.is_empty());
        assert!(cursor.is_none());
    }

    // ── default limit ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_messages_given_no_limit_when_list_then_default_50() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s.get_or_create_conversation(ua, ub).await.unwrap();
        // kirim 2 pesan, tanpa limit → default 50
        s.send_message(
            conv.id,
            ua,
            SendMessageInput {
                content: "P1".into(),
            },
        )
        .await
        .unwrap();
        let (msgs, _) = s
            .list_messages(
                conv.id,
                ListMessagesQuery {
                    limit: None,
                    before_id: None,
                    after_id: None,
                },
            )
            .await
            .unwrap();
        assert!(!msgs.is_empty());
    }
}
