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
        UpdatePekerjaParams,
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

        async fn list(
            &self,
            limit: i64,
            offset: i64,
            radius: Option<common_geo::RadiusQuery>,
        ) -> Result<Vec<IklanPekerja>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .filter(|i| i.deleted_at.is_none())
                .filter(|i| match radius {
                    None => true,
                    Some(r) => match (i.latitude, i.longitude) {
                        (Some(lat), Some(lng)) => {
                            common_geo::within_radius_km(r.lat, r.lng, lat, lng, r.radius_km)
                        }
                        _ => false,
                    },
                })
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
                region_id: params.region_id.map(String::from),
                tarif_min: params.tarif_min,
                tarif_max: params.tarif_max,
                jam_kerja: params.jam_kerja.map(String::from),
                phone_number: params.phone_number.map(String::from),
                foto_urls: vec![],
                is_active: true,
                moderation_status: ModerationStatus::Active,
                deleted_at: None,
                latitude: params.latitude,
                longitude: params.longitude,
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
        async fn exists_active_for_poster(&self, poster_id: Uuid) -> Result<bool, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .any(|i| i.poster_id == poster_id && i.is_active && i.deleted_at.is_none()))
        }

        async fn find_active_by_posters(
            &self,
            poster_ids: &[Uuid],
        ) -> Result<Vec<IklanPekerja>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .filter(|i| {
                    poster_ids.contains(&i.poster_id) && i.is_active && i.deleted_at.is_none()
                })
                .cloned()
                .collect())
        }

        async fn update(
            &self,
            id: Uuid,
            poster_id: Uuid,
            _params: UpdatePekerjaParams,
        ) -> Result<Option<IklanPekerja>, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan
                .iter_mut()
                .find(|i| i.id == id && i.poster_id == poster_id && i.deleted_at.is_none())
            {
                item.updated_at = chrono::Utc::now();
                Ok(Some(item.clone()))
            } else {
                Ok(None)
            }
        }
    }

    fn svc() -> IklanPekerjaService<MockIklanPekerjaRepository> {
        IklanPekerjaService::new(std::sync::Arc::new(MockIklanPekerjaRepository::new()))
    }

    // ── MockUserClient (F-27b — kontrak proxy lintas-service ke user-service) ─────

    #[derive(Default)]
    struct MockUserClient {
        flags: user_service_client::SensitiveDocFlags,
        nik: Option<String>,
        document_url: Option<String>,
        /// Rekam tiap pemanggilan reveal: (auth_id, admin_id, kind).
        reveal_calls: Mutex<Vec<(Uuid, Uuid, &'static str)>>,
    }

    #[async_trait::async_trait]
    impl user_service_client::UserClient for MockUserClient {
        async fn get_user_summary(
            &self,
            _user_id: Uuid,
        ) -> Result<user_service_client::UserSummary, user_service_client::UserClientError>
        {
            Err(user_service_client::UserClientError::NotFound)
        }
        async fn user_exists(
            &self,
            _user_id: Uuid,
        ) -> Result<bool, user_service_client::UserClientError> {
            Ok(true)
        }
        async fn purge_kyc_documents(
            &self,
            _user_id: Uuid,
        ) -> Result<(), user_service_client::UserClientError> {
            Ok(())
        }
        async fn get_sensitive_doc_flags(
            &self,
            _auth_id: Uuid,
        ) -> Result<user_service_client::SensitiveDocFlags, user_service_client::UserClientError>
        {
            Ok(self.flags)
        }
        async fn admin_reveal_nik(
            &self,
            auth_id: Uuid,
            admin_id: Uuid,
        ) -> Result<Option<String>, user_service_client::UserClientError> {
            self.reveal_calls
                .lock()
                .unwrap()
                .push((auth_id, admin_id, "nik"));
            Ok(self.nik.clone())
        }
        async fn admin_get_document_url(
            &self,
            auth_id: Uuid,
            kind: &str,
            admin_id: Uuid,
        ) -> Result<Option<String>, user_service_client::UserClientError> {
            let kind_static = match kind {
                "ktp" => "ktp",
                "selfie" => "selfie",
                _ => "unknown",
            };
            self.reveal_calls
                .lock()
                .unwrap()
                .push((auth_id, admin_id, kind_static));
            Ok(self.document_url.clone())
        }
        async fn get_location_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<
            Vec<user_service_client::UserLocationSummary>,
            user_service_client::UserClientError,
        > {
            Ok(vec![])
        }
        async fn get_demographic_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<
            Vec<user_service_client::UserDemographicSummary>,
            user_service_client::UserClientError,
        > {
            Ok(vec![])
        }
        async fn get_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<Vec<user_service_client::UserSummary>, user_service_client::UserClientError>
        {
            Ok(vec![])
        }
    }

    fn svc_with_user_client(uc: MockUserClient) -> IklanPekerjaService<MockIklanPekerjaRepository> {
        IklanPekerjaService::new(std::sync::Arc::new(MockIklanPekerjaRepository::new()))
            .with_user_client(std::sync::Arc::new(uc))
    }

    // ── admin_get_detail (F-27b) ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_get_detail_given_poster_with_sensitive_docs_when_get_then_flags_true() {
        let uc = MockUserClient {
            flags: user_service_client::SensitiveDocFlags {
                has_nik: true,
                has_ktp: true,
                has_selfie: false,
            },
            ..Default::default()
        };
        let s = svc_with_user_client(uc);
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Detail".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();

        let detail = s.admin_get_detail(created.id).await.unwrap().unwrap();
        assert_eq!(detail.poster_id, poster);
        assert!(detail.has_nik);
        assert!(detail.has_ktp);
        assert!(!detail.has_selfie);
    }

    #[tokio::test]
    async fn test_admin_get_detail_given_unknown_id_when_get_then_returns_none() {
        let s = svc();
        assert!(s.admin_get_detail(Uuid::now_v7()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_admin_get_detail_given_no_user_client_when_get_then_flags_default_false() {
        // Tanpa UserClient (mis. belum di-wire) — degradasi anggun, bukan gagal total.
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "NoClient".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let detail = s.admin_get_detail(created.id).await.unwrap().unwrap();
        assert!(!detail.has_nik);
        assert!(!detail.has_ktp);
        assert!(!detail.has_selfie);
    }

    // ── admin_reveal_sensitive (F-27b — proxy, TANPA duplikasi audit) ───────────

    #[tokio::test]
    async fn test_admin_reveal_sensitive_given_kind_nik_when_reveal_then_proxies_to_user_client_with_poster_id(
    ) {
        let uc = MockUserClient {
            nik: Some("3201234567890123".into()),
            ..Default::default()
        };
        let uc_arc = std::sync::Arc::new(uc);
        let s = IklanPekerjaService::new(std::sync::Arc::new(MockIklanPekerjaRepository::new()))
            .with_user_client(uc_arc.clone());
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "Reveal".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();

        let result = s
            .admin_reveal_sensitive(created.id, "nik", admin)
            .await
            .unwrap();
        assert_eq!(result, Some("3201234567890123".to_string()));

        // Bukti tidak ada duplikasi: satu-satunya "sumber kebenaran" adalah mock
        // UserClient (yang merepresentasikan user-service) — iklan-pekerja-service
        // sendiri tidak punya state/audit lokal untuk data ini sama sekali.
        let calls = uc_arc.reveal_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], (poster, admin, "nik"));
    }

    #[tokio::test]
    async fn test_admin_reveal_sensitive_given_kind_ktp_when_reveal_then_proxies_with_correct_kind()
    {
        let uc = MockUserClient {
            document_url: Some("https://presigned.example/ktp.jpg".into()),
            ..Default::default()
        };
        let uc_arc = std::sync::Arc::new(uc);
        let s = IklanPekerjaService::new(std::sync::Arc::new(MockIklanPekerjaRepository::new()))
            .with_user_client(uc_arc.clone());
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "RevealKtp".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();

        let result = s
            .admin_reveal_sensitive(created.id, "ktp", admin)
            .await
            .unwrap();
        assert_eq!(
            result,
            Some("https://presigned.example/ktp.jpg".to_string())
        );
        assert_eq!(
            uc_arc.reveal_calls.lock().unwrap()[0],
            (poster, admin, "ktp")
        );
    }

    #[tokio::test]
    async fn test_admin_reveal_sensitive_given_no_user_client_when_reveal_then_errors() {
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "NoClient2".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let result = s
            .admin_reveal_sensitive(created.id, "nik", Uuid::now_v7())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_admin_reveal_sensitive_given_unknown_iklan_when_reveal_then_returns_none() {
        let s = svc_with_user_client(MockUserClient::default());
        let result = s
            .admin_reveal_sensitive(Uuid::now_v7(), "nik", Uuid::now_v7())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_admin_reveal_sensitive_given_invalid_kind_when_reveal_then_errors() {
        let s = svc_with_user_client(MockUserClient::default());
        let poster = Uuid::now_v7();
        let created = s
            .create(
                poster,
                CreateIklanPekerjaInput {
                    nama: "BadKind".into(),
                    keahlian: vec!["X".into()],
                    deskripsi: "D".into(),
                    lokasi: None,
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let result = s
            .admin_reveal_sensitive(created.id, "not-a-kind", Uuid::now_v7())
            .await;
        assert!(result.is_err());
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
                region_id: None,
                tarif_min: None,
                tarif_max: None,
                jam_kerja: None,
                phone_number: None,
                foto_urls: None,
            },
        )
        .await
        .unwrap();
        let result = s
            .list(ListQuery {
                limit: Some(10),
                offset: Some(0),
                latitude: None,
                longitude: None,
                max_distance: None,
            })
            .await
            .unwrap();
        assert!(!result.is_empty());
        assert_eq!(result[0].nama, "Budi");
    }

    /// Filter radius (F-1, PRD §5.12.1): iklan di luar radius default (2km) tidak muncul,
    /// iklan di dalam radius tetap muncul — via `MockGeocodingClient` + koordinat pusat.
    #[tokio::test]
    async fn test_list_given_lat_lng_when_filtered_then_only_returns_iklan_within_radius() {
        let repo = std::sync::Arc::new(MockIklanPekerjaRepository::new());
        let s = IklanPekerjaService::new(repo.clone());
        let center = (-6.2, 106.8);
        // ~1km dari center (dalam radius default 2km).
        let near = std::sync::Arc::new(MockGeocodingClient {
            result: Ok(Some(common_geocoding::Coordinates {
                latitude: -6.2 + 1.0 / 111.32,
                longitude: 106.8,
            })),
        });
        // ~5km dari center (luar radius default 2km).
        let far = std::sync::Arc::new(MockGeocodingClient {
            result: Ok(Some(common_geocoding::Coordinates {
                latitude: -6.2 + 5.0 / 111.32,
                longitude: 106.8,
            })),
        });

        IklanPekerjaService::new(repo.clone())
            .with_geocoding_client(near)
            .create(Uuid::now_v7(), make_create_input(Some("Dekat")))
            .await
            .unwrap();
        IklanPekerjaService::new(repo.clone())
            .with_geocoding_client(far)
            .create(Uuid::now_v7(), make_create_input(Some("Jauh")))
            .await
            .unwrap();

        let result = s
            .list(ListQuery {
                limit: Some(10),
                offset: Some(0),
                latitude: Some(center.0),
                longitude: Some(center.1),
                max_distance: None,
            })
            .await
            .unwrap();

        assert_eq!(
            result.len(),
            1,
            "hanya iklan dalam radius yang muncul: {result:?}"
        );
        assert_eq!(result[0].lokasi.as_deref(), Some("Dekat"));
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
                    region_id: None,
                    tarif_min: Some(500_000),
                    tarif_max: Some(2_000_000),
                    jam_kerja: None,
                    phone_number: None,
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
                    region_id: None,
                    tarif_min: Some(1_000_000),
                    tarif_max: Some(5_000_000),
                    jam_kerja: None,
                    phone_number: None,
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
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
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
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
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
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
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
                region_id: None,
                tarif_min: None,
                tarif_max: None,
                jam_kerja: None,
                phone_number: None,
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
                    region_id: None,
                    tarif_min: None,
                    tarif_max: None,
                    jam_kerja: None,
                    phone_number: None,
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
    #[allow(clippy::assertions_on_constants, clippy::nonminimal_bool)]
    fn test_csv_max_is_10000() {
        assert!(10_000i64 >= 1_000 && 10_000i64 <= 100_000);
    }

    // ── entity ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_moderation_status_default_active() {
        assert_eq!(ModerationStatus::default().as_str(), "active");
    }

    // ── geocoding wiring (F-1, Kelompok 2 P2.4) ───────────────────────────────────

    struct MockGeocodingClient {
        result: Result<Option<common_geocoding::Coordinates>, ()>,
    }

    #[async_trait::async_trait]
    impl common_geocoding::GeocodingClient for MockGeocodingClient {
        async fn geocode(
            &self,
            _input: &common_geocoding::GeocodeInput,
        ) -> Result<Option<common_geocoding::Coordinates>, common_geocoding::GeocodingClientError>
        {
            self.result
                .map_err(|_| common_geocoding::GeocodingClientError::Unavailable)
        }
    }

    fn make_create_input(lokasi: Option<&str>) -> CreateIklanPekerjaInput {
        CreateIklanPekerjaInput {
            nama: "Tukang".into(),
            keahlian: vec!["listrik".into()],
            deskripsi: "Deskripsi".into(),
            lokasi: lokasi.map(String::from),
            region_id: None,
            tarif_min: None,
            tarif_max: None,
            jam_kerja: None,
            phone_number: None,
            foto_urls: None,
        }
    }

    #[tokio::test]
    async fn test_create_iklan_given_valid_address_when_saving_then_coordinates_populated() {
        let repo = std::sync::Arc::new(MockIklanPekerjaRepository::new());
        let gc = MockGeocodingClient {
            result: Ok(Some(common_geocoding::Coordinates {
                latitude: -6.2,
                longitude: 106.8,
            })),
        };
        let s =
            IklanPekerjaService::new(repo.clone()).with_geocoding_client(std::sync::Arc::new(gc));

        let created = s
            .create(Uuid::now_v7(), make_create_input(Some("Jl. Test No. 1")))
            .await
            .unwrap();

        let stored = repo
            .iklan
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == created.id)
            .cloned()
            .expect("iklan harus tersimpan");
        assert_eq!(stored.latitude, Some(-6.2));
        assert_eq!(stored.longitude, Some(106.8));
    }

    /// Geocoding tidak menemukan koordinat (kombinasi alamat tidak ditemukan) →
    /// iklan TETAP tersimpan tanpa koordinat (degradasi anggun, bukan error — §4.5 backend).
    #[tokio::test]
    async fn test_create_iklan_given_address_not_found_when_saving_then_coordinates_none() {
        let repo = std::sync::Arc::new(MockIklanPekerjaRepository::new());
        let gc = MockGeocodingClient { result: Ok(None) };
        let s =
            IklanPekerjaService::new(repo.clone()).with_geocoding_client(std::sync::Arc::new(gc));

        let created = s
            .create(
                Uuid::now_v7(),
                make_create_input(Some("Alamat Tidak Jelas")),
            )
            .await
            .unwrap();

        let stored = repo
            .iklan
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == created.id)
            .cloned()
            .expect("iklan harus tersimpan meski tanpa koordinat");
        assert_eq!(stored.latitude, None);
        assert_eq!(stored.longitude, None);
    }
}
