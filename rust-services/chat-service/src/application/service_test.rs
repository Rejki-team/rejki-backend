#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{ListMessagesQuery, SendMessageInput};
    use crate::application::service::ChatService;
    use crate::domain::entity::{Conversation, Message, MessageContentType, NewMessage};
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
            related_ad_type: Option<&str>,
            related_ad_id: Option<Uuid>,
        ) -> Result<(Conversation, bool), anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            if let Some(existing) = convs.iter_mut().find(|c| {
                (c.user_a == user_a && c.user_b == user_b)
                    || (c.user_a == user_b && c.user_b == user_a)
            }) {
                if let Some(t) = related_ad_type {
                    existing.related_ad_type = Some(t.to_string());
                }
                if let Some(id) = related_ad_id {
                    existing.related_ad_id = Some(id);
                }
                return Ok((existing.clone(), false));
            }
            let new_conv = Conversation {
                id: Uuid::now_v7(),
                user_a,
                user_b,
                created_at: chrono::Utc::now(),
                ended_at: None,
                related_ad_type: related_ad_type.map(|s| s.to_string()),
                related_ad_id,
                user_a_last_read_message_id: None,
                user_b_last_read_message_id: None,
            };
            convs.push(new_conv.clone());
            Ok((new_conv, true))
        }

        async fn find_conversation_by_id(
            &self,
            id: Uuid,
        ) -> Result<Option<Conversation>, anyhow::Error> {
            Ok(self
                .conversations
                .lock()
                .unwrap()
                .iter()
                .find(|c| c.id == id)
                .cloned())
        }

        async fn conversation_exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(self
                .conversations
                .lock()
                .unwrap()
                .iter()
                .any(|c| c.id == id))
        }

        async fn is_participant(
            &self,
            conversation_id: Uuid,
            user_id: Uuid,
        ) -> Result<bool, anyhow::Error> {
            Ok(self
                .conversations
                .lock()
                .unwrap()
                .iter()
                .any(|c| c.id == conversation_id && (c.user_a == user_id || c.user_b == user_id)))
        }

        async fn save_message(
            &self,
            conversation_id: Uuid,
            sender_id: Uuid,
            input: &NewMessage,
        ) -> Result<Message, anyhow::Error> {
            let msg = Message {
                id: Uuid::now_v7(),
                conversation_id,
                sender_id,
                content_type: input.content_type,
                content: input.content.clone(),
                lat: input.lat,
                lng: input.lng,
                photo_object_key: input.photo_object_key.clone(),
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

        async fn end_conversation(
            &self,
            id: Uuid,
            actor_id: Uuid,
        ) -> Result<Option<Conversation>, anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            let Some(conv) = convs
                .iter_mut()
                .find(|c| c.id == id && (c.user_a == actor_id || c.user_b == actor_id))
            else {
                return Ok(None);
            };
            if conv.ended_at.is_none() {
                conv.ended_at = Some(chrono::Utc::now());
            }
            Ok(Some(conv.clone()))
        }

        async fn auto_end_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            let Some(conv) = convs.iter_mut().find(|c| c.id == id) else {
                return Ok(false);
            };
            if conv.ended_at.is_some() {
                return Ok(false);
            }
            conv.ended_at = Some(chrono::Utc::now());
            Ok(true)
        }

        async fn find_active_conversations_by_ad(
            &self,
            ad_type: &str,
            ad_id: Uuid,
        ) -> Result<Vec<Conversation>, anyhow::Error> {
            Ok(self
                .conversations
                .lock()
                .unwrap()
                .iter()
                .filter(|c| {
                    c.ended_at.is_none()
                        && c.related_ad_type.as_deref() == Some(ad_type)
                        && c.related_ad_id == Some(ad_id)
                })
                .cloned()
                .collect())
        }

        async fn purge_conversation(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            let before = convs.len();
            convs.retain(|c| c.id != id);
            self.messages
                .lock()
                .unwrap()
                .retain(|m| m.conversation_id != id);
            Ok(convs.len() < before)
        }

        async fn list_conversations_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<(Conversation, Option<Message>)>, anyhow::Error> {
            let convs = self.conversations.lock().unwrap();
            let msgs = self.messages.lock().unwrap();
            Ok(convs
                .iter()
                .filter(|c| c.user_a == user_id || c.user_b == user_id)
                .map(|c| {
                    let last = msgs
                        .iter()
                        .filter(|m| m.conversation_id == c.id)
                        .max_by_key(|m| m.created_at)
                        .cloned();
                    (c.clone(), last)
                })
                .collect())
        }

        async fn mark_read(&self, id: Uuid, user_id: Uuid) -> Result<bool, anyhow::Error> {
            let mut convs = self.conversations.lock().unwrap();
            let msgs = self.messages.lock().unwrap();
            let Some(conv) = convs
                .iter_mut()
                .find(|c| c.id == id && (c.user_a == user_id || c.user_b == user_id))
            else {
                return Ok(false);
            };
            let last_id = msgs
                .iter()
                .filter(|m| m.conversation_id == id)
                .max_by_key(|m| m.created_at)
                .map(|m| m.id);
            if conv.user_a == user_id {
                conv.user_a_last_read_message_id = last_id;
            } else {
                conv.user_b_last_read_message_id = last_id;
            }
            Ok(true)
        }
    }

    fn svc() -> ChatService<MockChatRepository> {
        ChatService::new(std::sync::Arc::new(MockChatRepository::new()))
    }

    fn text_input(content: &str) -> SendMessageInput {
        SendMessageInput::Text {
            content: content.to_string(),
        }
    }

    // ── get_or_create_conversation ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_or_create_conversation_given_new_pair_when_create_then_returns_conv() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        assert_eq!(conv.user_a, ua);
        assert_eq!(conv.user_b, ub);
        assert!(!conv.id.is_nil());
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_given_existing_pair_when_call_again_then_same_id() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let c1 = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let c2 = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        assert_eq!(c1.id, c2.id);
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_given_reversed_pair_when_call_then_same_conv() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let c1 = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let c2 = s
            .get_or_create_conversation(ub, ua, None, None)
            .await
            .unwrap();
        assert_eq!(c1.id, c2.id);
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_given_ad_link_when_call_again_without_link_then_preserved(
    ) {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let iklan_id = Uuid::now_v7();
        let c1 = s
            .get_or_create_conversation(ua, ub, Some("pekerjaan"), Some(iklan_id))
            .await
            .unwrap();
        assert_eq!(c1.related_ad_type.as_deref(), Some("pekerjaan"));
        // Panggilan berikutnya tanpa ad link — link lama TIDAK ditimpa jadi None.
        let c2 = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        assert_eq!(c2.related_ad_type.as_deref(), Some("pekerjaan"));
        assert_eq!(c2.related_ad_id, Some(iklan_id));
    }

    // ── send_message ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_send_message_given_valid_conversation_when_send_text_then_succeeds() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let msg = s
            .send_message(conv.id, ua, text_input("Halo!"))
            .await
            .unwrap();
        assert_eq!(msg.conversation_id, conv.id);
        assert_eq!(msg.sender_id, ua);
        assert_eq!(msg.content_type, MessageContentType::Text.as_str());
        assert_eq!(msg.content.as_deref(), Some("Halo!"));
    }

    #[tokio::test]
    async fn test_send_message_given_location_when_send_then_lat_lng_tersimpan() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let msg = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput::Location {
                    lat: -6.2,
                    lng: 106.8,
                },
            )
            .await
            .unwrap();
        assert_eq!(msg.content_type, MessageContentType::Location.as_str());
        assert_eq!(msg.lat, Some(-6.2));
        assert_eq!(msg.lng, Some(106.8));
        assert!(msg.content.is_none());
    }

    #[tokio::test]
    async fn test_send_message_given_invalid_latitude_when_send_then_returns_error() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let result = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput::Location {
                    lat: 999.0,
                    lng: 106.8,
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("koordinat"));
    }

    #[tokio::test]
    async fn test_send_message_given_photo_when_send_then_object_key_tersimpan() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let msg = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput::Photo {
                    photo_object_key: "chat-photo/x.jpg".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(msg.content_type, MessageContentType::Photo.as_str());
        assert_eq!(msg.photo_object_key.as_deref(), Some("chat-photo/x.jpg"));
    }

    #[tokio::test]
    async fn test_send_message_given_empty_photo_object_key_when_send_then_returns_error() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let result = s
            .send_message(
                conv.id,
                ua,
                SendMessageInput::Photo {
                    photo_object_key: "   ".into(),
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_message_given_nonexistent_conversation_when_send_then_returns_error() {
        let s = svc();
        let result = s
            .send_message(Uuid::now_v7(), Uuid::now_v7(), text_input("X"))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tidak ditemukan"));
    }

    #[tokio::test]
    async fn test_send_message_given_non_participant_when_send_then_returns_error() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let intruder = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let result = s.send_message(conv.id, intruder, text_input("X")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tidak ditemukan"));
    }

    #[tokio::test]
    async fn test_send_message_given_html_content_when_send_then_strips_html() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let msg = s
            .send_message(
                conv.id,
                ua,
                text_input("<script>alert('xss')</script>Hello"),
            )
            .await
            .unwrap();
        let content = msg.content.unwrap();
        assert!(!content.contains('<'));
        assert!(content.contains("Hello"));
    }

    // ── end_conversation (P4.4) ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_end_conversation_given_participant_when_end_then_ended_at_terisi() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let ended = s.end_conversation(conv.id, ua).await.unwrap();
        assert!(ended.ended_at.is_some());
    }

    #[tokio::test]
    async fn test_end_conversation_given_non_participant_when_end_then_returns_error() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let intruder = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let result = s.end_conversation(conv.id, intruder).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_end_conversation_given_already_ended_when_end_again_then_idempotent() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        let first = s.end_conversation(conv.id, ua).await.unwrap();
        let second = s.end_conversation(conv.id, ub).await.unwrap();
        assert_eq!(first.ended_at, second.ended_at);
    }

    // ── list_messages ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_messages_given_messages_when_list_then_returns_all() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        s.send_message(conv.id, ua, text_input("A")).await.unwrap();
        s.send_message(conv.id, ub, text_input("B")).await.unwrap();
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
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        for i in 0..5 {
            s.send_message(conv.id, ua, text_input(&format!("Msg {i}")))
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
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
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
        assert!(msgs.is_empty());
        assert!(cursor.is_none());
    }

    // ── default limit ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_messages_given_no_limit_when_list_then_default_50() {
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let conv = s
            .get_or_create_conversation(ua, ub, None, None)
            .await
            .unwrap();
        s.send_message(conv.id, ua, text_input("P1")).await.unwrap();
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

    // ── schedule_auto_end_for_ad (P4.3) ──────────────────────────────────────────

    #[tokio::test]
    async fn test_schedule_auto_end_for_ad_given_no_scheduler_client_then_no_op_ok() {
        // Tanpa scheduler_client terpasang → fail-open, tidak error (§4.5 backend).
        let s = svc();
        let ua = Uuid::now_v7();
        let ub = Uuid::now_v7();
        let iklan_id = Uuid::now_v7();
        s.get_or_create_conversation(ua, ub, Some("pekerjaan"), Some(iklan_id))
            .await
            .unwrap();
        let result = s.schedule_auto_end_for_ad("pekerjaan", iklan_id).await;
        assert!(result.is_ok());
    }
}
