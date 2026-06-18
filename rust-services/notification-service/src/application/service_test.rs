#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{RegisterDeviceTokenInput, SendNotificationInput};
    use crate::application::service::NotificationService;
    use crate::domain::entity::{DeviceToken, Notification};
    use crate::domain::repository::NotificationRepository;

    // ── MockNotificationRepository ──────────────────────────────────────────────

    struct MockNotificationRepository {
        notifications: Mutex<Vec<Notification>>,
        device_tokens: Mutex<Vec<DeviceToken>>,
    }

    impl MockNotificationRepository {
        fn new() -> Self {
            Self {
                notifications: Mutex::new(vec![]),
                device_tokens: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl NotificationRepository for MockNotificationRepository {
        async fn save(
            &self,
            recipient_id: Uuid,
            title: &str,
            body: &str,
            data: Option<serde_json::Value>,
        ) -> Result<Notification, anyhow::Error> {
            let notif = Notification {
                id: Uuid::now_v7(),
                recipient_id,
                title: title.to_string(),
                body: body.to_string(),
                data,
                is_read: false,
                created_at: chrono::Utc::now(),
            };
            self.notifications.lock().unwrap().push(notif.clone());
            Ok(notif)
        }

        async fn list_for_user(
            &self,
            recipient_id: Uuid,
            _limit: i64,
        ) -> Result<Vec<Notification>, anyhow::Error> {
            Ok(self
                .notifications
                .lock()
                .unwrap()
                .iter()
                .filter(|n| n.recipient_id == recipient_id)
                .cloned()
                .collect())
        }

        async fn mark_read(&self, id: Uuid, recipient_id: Uuid) -> Result<(), anyhow::Error> {
            let mut notifs = self.notifications.lock().unwrap();
            if let Some(n) = notifs
                .iter_mut()
                .find(|n| n.id == id && n.recipient_id == recipient_id)
            {
                n.is_read = true;
            }
            Ok(())
        }

        async fn register_device_token(
            &self,
            user_id: Uuid,
            token: &str,
            platform: &str,
        ) -> Result<DeviceToken, anyhow::Error> {
            let dt = DeviceToken {
                id: Uuid::now_v7(),
                user_id,
                token: token.to_string(),
                platform: platform.to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.device_tokens.lock().unwrap().push(dt.clone());
            Ok(dt)
        }

        async fn delete_device_token(
            &self,
            token: &str,
            user_id: Uuid,
        ) -> Result<(), anyhow::Error> {
            self.device_tokens
                .lock()
                .unwrap()
                .retain(|dt| !(dt.token == token && dt.user_id == user_id));
            Ok(())
        }

        async fn list_device_tokens_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<Vec<DeviceToken>, anyhow::Error> {
            Ok(self
                .device_tokens
                .lock()
                .unwrap()
                .iter()
                .filter(|dt| dt.user_id == user_id)
                .cloned()
                .collect())
        }
    }

    fn svc() -> NotificationService<MockNotificationRepository> {
        let repo = std::sync::Arc::new(MockNotificationRepository::new());
        NotificationService::new(repo)
    }

    // ── send ────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_send_given_valid_input_when_send_then_saves_notification() {
        let s = svc();
        let recipient = Uuid::now_v7();
        s.send(SendNotificationInput {
            recipient_id: recipient,
            title: "Test".into(),
            body: "Body text".into(),
            data: None,
        })
        .await
        .unwrap();

        let list = s.list_for_user(recipient, None).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Test");
        assert!(!list[0].is_read);
    }

    // ── list_for_user ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_for_user_given_notifications_when_list_then_returns_all() {
        let s = svc();
        let user = Uuid::now_v7();
        s.send(SendNotificationInput {
            recipient_id: user,
            title: "A".into(),
            body: "B".into(),
            data: None,
        })
        .await
        .unwrap();
        s.send(SendNotificationInput {
            recipient_id: user,
            title: "C".into(),
            body: "D".into(),
            data: None,
        })
        .await
        .unwrap();
        // Notification for different user should NOT appear
        s.send(SendNotificationInput {
            recipient_id: Uuid::now_v7(),
            title: "Other".into(),
            body: "X".into(),
            data: None,
        })
        .await
        .unwrap();

        let list = s.list_for_user(user, None).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    // ── mark_read ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mark_read_given_unread_notification_when_mark_then_becomes_read() {
        let s = svc();
        let user = Uuid::now_v7();
        s.send(SendNotificationInput {
            recipient_id: user,
            title: "Read me".into(),
            body: "X".into(),
            data: None,
        })
        .await
        .unwrap();
        let list = s.list_for_user(user, None).await.unwrap();
        assert!(!list[0].is_read);

        s.mark_read(list[0].id, user).await.unwrap();
        let updated = s.list_for_user(user, None).await.unwrap();
        assert!(updated[0].is_read);
    }

    // ── send_bulk ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_send_bulk_given_multiple_recipients_when_send_then_saves_all() {
        let s = svc();
        let u1 = Uuid::now_v7();
        let u2 = Uuid::now_v7();
        s.send_bulk(vec![u1, u2], "Bulk", "Body").await.unwrap();

        assert_eq!(s.list_for_user(u1, None).await.unwrap().len(), 1);
        assert_eq!(s.list_for_user(u2, None).await.unwrap().len(), 1);
    }

    // ── register_device_token ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_register_device_token_given_valid_platform_when_register_then_succeeds() {
        let s = svc();
        let user = Uuid::now_v7();
        let dt = s
            .register_device_token(
                RegisterDeviceTokenInput {
                    token: "fcm-token-abc".into(),
                    platform: "android".into(),
                },
                user,
            )
            .await
            .unwrap();
        assert_eq!(dt.token, "fcm-token-abc");
        assert_eq!(dt.platform, "android");
        assert_eq!(dt.user_id, user);
    }

    #[tokio::test]
    async fn test_register_device_token_given_ios_platform_when_register_then_succeeds() {
        let s = svc();
        let user = Uuid::now_v7();
        let dt = s
            .register_device_token(
                RegisterDeviceTokenInput {
                    token: "apns-token".into(),
                    platform: "ios".into(),
                },
                user,
            )
            .await
            .unwrap();
        assert_eq!(dt.platform, "ios");
    }

    #[tokio::test]
    async fn test_register_device_token_given_invalid_platform_when_register_then_returns_error() {
        let s = svc();
        let result = s
            .register_device_token(
                RegisterDeviceTokenInput {
                    token: "bad".into(),
                    platform: "windows".into(),
                },
                Uuid::now_v7(),
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("platform"));
    }

    // ── delete_device_token ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_device_token_given_existing_token_when_delete_then_removes() {
        let s = svc();
        let user = Uuid::now_v7();
        s.register_device_token(
            RegisterDeviceTokenInput {
                token: "delete-me".into(),
                platform: "web".into(),
            },
            user,
        )
        .await
        .unwrap();
        assert_eq!(s.list_device_tokens(user).await.unwrap().len(), 1);

        s.delete_device_token("delete-me", user).await.unwrap();
        assert!(s.list_device_tokens(user).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_delete_device_token_given_other_user_when_delete_then_token_remains() {
        let s = svc();
        let user = Uuid::now_v7();
        let other = Uuid::now_v7();
        s.register_device_token(
            RegisterDeviceTokenInput {
                token: "mine".into(),
                platform: "android".into(),
            },
            user,
        )
        .await
        .unwrap();
        s.delete_device_token("mine", other).await.unwrap(); // other user can't delete
        assert_eq!(s.list_device_tokens(user).await.unwrap().len(), 1);
    }

    // ── list_device_tokens ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_device_tokens_given_registered_tokens_when_list_then_returns_all() {
        let s = svc();
        let user = Uuid::now_v7();
        s.register_device_token(
            RegisterDeviceTokenInput {
                token: "t1".into(),
                platform: "android".into(),
            },
            user,
        )
        .await
        .unwrap();
        s.register_device_token(
            RegisterDeviceTokenInput {
                token: "t2".into(),
                platform: "web".into(),
            },
            user,
        )
        .await
        .unwrap();

        let other_user = Uuid::now_v7();
        s.register_device_token(
            RegisterDeviceTokenInput {
                token: "t3".into(),
                platform: "ios".into(),
            },
            other_user,
        )
        .await
        .unwrap();

        let tokens = s.list_device_tokens(user).await.unwrap();
        assert_eq!(tokens.len(), 2);
    }
}
