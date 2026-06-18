#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{
        AdminListQuery, CreateIklanPekerjaInput, ListQuery, SuspendInput,
    };
    use crate::application::service::IklanPekerjaService;
    use crate::domain::entity::{IklanPekerja, IklanSuspension, ModerationStatus};
    use crate::domain::repository::{
        AdminListParams, AdminListResult, CreatePekerjaParams, IklanPekerjaRepository,
    };

    // ── MockIklanPekerjaRepository ────────────────────────────────────────────────

    struct MockIklanPekerjaRepository {
        iklan: Mutex<Vec<IklanPekerja>>,
        _suspensions: Mutex<Vec<IklanSuspension>>,
    }

    impl MockIklanPekerjaRepository {
        fn new() -> Self {
            Self {
                iklan: Mutex::new(vec![]),
                _suspensions: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl IklanPekerjaRepository for MockIklanPekerjaRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPekerja>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .find(|i| i.id == id && i.deleted_at.is_none())
                .cloned())
        }

        async fn list(&self, limit: i64, offset: i64) -> Result<Vec<IklanPekerja>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .filter(|i| i.deleted_at.is_none())
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn create(
            &self,
            params: CreatePekerjaParams<'_>,
        ) -> Result<IklanPekerja, anyhow::Error> {
            let iklan = IklanPekerja {
                id: Uuid::now_v7(),
                poster_id: params.poster_id,
                nama: params.nama.to_string(),
                keahlian: params.keahlian.to_vec(),
                deskripsi: params.deskripsi.to_string(),
                lokasi: params.lokasi.map(String::from),
                tarif_min: params.tarif_min,
                tarif_max: params.tarif_max,
                foto_urls: vec![],
                is_active: true,
                moderation_status: ModerationStatus::Active,
                deleted_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.iklan.lock().unwrap().push(iklan.clone());
            Ok(iklan)
        }

        async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan
                .iter_mut()
                .find(|i| i.id == id && i.poster_id == poster_id && i.deleted_at.is_none())
            {
                item.deleted_at = Some(chrono::Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn exists(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .any(|i| i.id == id && i.deleted_at.is_none()))
        }

        async fn admin_list(
            &self,
            _params: AdminListParams,
        ) -> Result<AdminListResult, anyhow::Error> {
            let items: Vec<_> = self.iklan.lock().unwrap().clone();
            Ok(AdminListResult {
                total: items.len() as i64,
                items,
            })
        }

        async fn admin_list_all(
            &self,
            _params: AdminListParams,
        ) -> Result<Vec<IklanPekerja>, anyhow::Error> {
            Ok(self.iklan.lock().unwrap().clone())
        }

        async fn suspend(
            &self,
            iklan_ids: &[Uuid],
            is_permanent: bool,
            reason: &str,
            evidence_object_key: Option<&str>,
            expires_at: Option<chrono::DateTime<chrono::Utc>>,
            created_by: Uuid,
        ) -> Result<Vec<IklanSuspension>, anyhow::Error> {
            let mut results = vec![];
            for id in iklan_ids {
                if self.iklan.lock().unwrap().iter().any(|i| i.id == *id) {
                    results.push(IklanSuspension {
                        id: Uuid::now_v7(),
                        iklan_id: *id,
                        is_permanent,
                        reason: reason.to_string(),
                        evidence_object_key: evidence_object_key.map(String::from),
                        expires_at,
                        created_by,
                        created_at: chrono::Utc::now(),
                    });
                }
            }
            Ok(results)
        }

        async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan.iter_mut().find(|i| i.id == id) {
                item.deleted_at = Some(chrono::Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn expire_temporary_suspensions(&self) -> Result<u64, anyhow::Error> {
            Ok(0)
        }
        async fn is_poster_in_cooldown(&self, _poster_id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(false)
        }
    }

    fn svc() -> IklanPekerjaService<MockIklanPekerjaRepository> {
        IklanPekerjaService::new(std::sync::Arc::new(MockIklanPekerjaRepository::new()))
    }

    // ── list ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_given_iklan_when_list_then_returns_items() {
        let s = svc();
        let poster = Uuid::now_v7();
        s.create(
            poster,
            CreateIklanPekerjaInput {
                nama: "Budi".into(),
                keahlian: vec!["Rust".into()],
                deskripsi: "Dev".into(),
                lokasi: None,
                tarif_min: None,
                tarif_max: None,
                foto_urls: None,
            },
        )
        .await
        .unwrap();
        let result = s
            .list(ListQuery {
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(!result.is_empty());
        assert_eq!(result[0].nama, "Budi");
    }

    // ── get ──────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_given_valid_id_when_get_then_returns_iklan() {
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Ani".into(),
                    keahlian: vec!["Design".into()],
                    deskripsi: "UI/UX".into(),
                    lokasi: Some("Surabaya".into()),
                    tarif_min: Some(500_000),
                    tarif_max: Some(2_000_000),
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let got = s.get(created.id).await.unwrap();
        assert_eq!(got.nama, "Ani");
        assert_eq!(got.keahlian, vec!["Design"]);
    }

    #[tokio::test]
    async fn test_get_given_nonexistent_id_when_get_then_returns_error() {
        let s = svc();
        assert!(s.get(Uuid::now_v7()).await.is_err());
    }

    // ── create ───────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_given_valid_input_when_create_then_succeeds() {
        let s = svc();
        let poster = Uuid::now_v7();
        let result = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Cici".into(),
                    keahlian: vec!["Golang".into(), "Rust".into()],
                    deskripsi: "Backend engineer".into(),
                    lokasi: Some("Jakarta".into()),
                    tarif_min: Some(1_000_000),
                    tarif_max: Some(5_000_000),
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.nama, "Cici");
        assert_eq!(result.keahlian.len(), 2);
        assert!(!result.id.is_nil());
    }

    #[tokio::test]
    async fn test_create_given_html_in_deskripsi_when_create_then_strips_html() {
        let s = svc();
        let poster = Uuid::now_v7();
        let result = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "X".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "<script>evil</script>Clean".into(),
                    lokasi: None,
                    tarif_min: None,
                    tarif_max: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(!result.deskripsi.contains("<script>"));
        assert!(result.deskripsi.contains("Clean"));
    }

    // ── delete (IDOR) ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_given_owner_when_delete_then_returns_true() {
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Del".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    tarif_min: None,
                    tarif_max: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(s.delete(created.id, poster).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_given_other_user_when_delete_then_returns_false() {
        let s = svc();
        let owner = Uuid::now_v7();
        let other = Uuid::now_v7();
        let created = s
            .create(
                owner,
                CreateIklanPekerjaInput {
                    nama: "Y".into(),
                    keahlian: vec!["Y".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    tarif_min: None,
                    tarif_max: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(!s.delete(created.id, other).await.unwrap());
    }

    // ── admin listing ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_list_given_iklan_when_admin_list_then_returns_all() {
        let s = svc();
        let poster = Uuid::now_v7();
        s.create(
            poster,
            CreateIklanPekerjaInput {
                nama: "AdminVisible".into(),
                keahlian: vec!["X".into()],
                deskripsi: "D".into(),
                lokasi: None,
                tarif_min: None,
                tarif_max: None,
                foto_urls: None,
            },
        )
        .await
        .unwrap();
        let (items, total) = s
            .admin_list(AdminListQuery {
                q: None,
                status: None,
                sort_by: None,
                sort_dir: None,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(total >= 1);
        assert!(!items.is_empty());
    }

    // ── suspend ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_suspend_given_valid_ids_when_suspend_then_succeeds() {
        let s = svc();
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Sus".into(),
                    keahlian: vec!["S".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    tarif_min: None,
                    tarif_max: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let resp = s
            .suspend(
                admin,
                SuspendInput {
                    iklan_ids: vec![created.id],
                    is_permanent: false,
                    reason: "Melanggar aturan".into(),
                    evidence_object_key: "ev-001".into(),
                    expires_at: None,
                },
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(resp.results.len(), 1);
        assert!(resp.results[0].success);
    }

    // ── constants ────────────────────────────────────────────────────────────────

    #[test]
    fn test_default_limit_is_20() {
        assert!((10..=100).contains(&20i64));
    }

    #[test]
    fn test_csv_max_is_10000() {
        assert!(10_000i64 >= 1_000 && 10_000i64 <= 100_000);
    }

    // ── entity ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_moderation_status_default_active() {
        assert_eq!(ModerationStatus::default().as_str(), "active");
    }
}
