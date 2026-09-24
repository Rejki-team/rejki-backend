#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{
        AdminListQuery, CreateIklanBarangBekasInput, ListQuery, SetujuiBiderInput, SuspendInput,
    };
    use crate::application::service::IklanBarangBekasService;
    use crate::domain::entity::{
        AvailabilityStatus, Bider, BiderStatus, IklanBarangBekas, IklanSuspension, ModerationStatus,
    };
    use crate::domain::repository::{
        AdminListParams, AdminListResult, CreateBarangBekasParams, IklanBarangBekasRepository,
        UpdateBarangBekasParams,
    };

    // ── MockIklanBarangBekasRepository ────────────────────────────────────────────

    struct MockIklanBarangBekasRepository {
        iklan: Mutex<Vec<IklanBarangBekas>>,
        bider: Mutex<Vec<Bider>>,
    }

    impl MockIklanBarangBekasRepository {
        fn new() -> Self {
            Self {
                iklan: Mutex::new(vec![]),
                bider: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl IklanBarangBekasRepository for MockIklanBarangBekasRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanBarangBekas>, anyhow::Error> {
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
        ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
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
            params: CreateBarangBekasParams<'_>,
        ) -> Result<IklanBarangBekas, anyhow::Error> {
            let iklan = IklanBarangBekas {
                id: Uuid::now_v7(),
                seller_id: params.seller_id,
                judul: params.judul.to_string(),
                deskripsi: params.deskripsi.to_string(),
                jenis_barang: params.jenis_barang.to_string(),
                jumlah: params.jumlah,
                lokasi_pengambilan: params.lokasi_pengambilan.to_string(),
                lokasi: params.lokasi.map(String::from),
                region_id: params.region_id.map(String::from),
                foto_urls: params.foto_urls.to_vec(),
                availability_status: AvailabilityStatus::Tersedia,
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

        async fn mark_taken(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan
                .iter_mut()
                .find(|i| i.id == id && i.seller_id == seller_id && i.deleted_at.is_none())
            {
                if item.availability_status == AvailabilityStatus::SudahDiambil {
                    return Ok(false);
                }
                item.availability_status = AvailabilityStatus::SudahDiambil;
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn delete(&self, id: Uuid, seller_id: Uuid) -> Result<bool, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan
                .iter_mut()
                .find(|i| i.id == id && i.seller_id == seller_id && i.deleted_at.is_none())
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
        ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
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

        async fn update(
            &self,
            id: Uuid,
            seller_id: Uuid,
            _params: UpdateBarangBekasParams,
        ) -> Result<Option<IklanBarangBekas>, anyhow::Error> {
            let mut iklan = self.iklan.lock().unwrap();
            if let Some(item) = iklan
                .iter_mut()
                .find(|i| i.id == id && i.seller_id == seller_id && i.deleted_at.is_none())
            {
                item.updated_at = chrono::Utc::now();
                Ok(Some(item.clone()))
            } else {
                Ok(None)
            }
        }

        async fn list_by_seller(
            &self,
            seller_id: Uuid,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .filter(|i| i.seller_id == seller_id && i.deleted_at.is_none())
                .skip(offset as usize)
                .take(limit as usize)
                .cloned()
                .collect())
        }

        async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
            Ok(self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .filter(|i| ids.contains(&i.id))
                .cloned()
                .collect())
        }

        // ── Bider (F-15, Kelompok 3 Phase 3) ────────────────────────────────────

        async fn find_bider_by_id(&self, id: Uuid) -> Result<Option<Bider>, anyhow::Error> {
            Ok(self
                .bider
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned())
        }

        async fn create_bider(
            &self,
            iklan_id: Uuid,
            peminat_id: Uuid,
        ) -> Result<Bider, anyhow::Error> {
            let b = Bider {
                id: Uuid::now_v7(),
                iklan_id,
                peminat_id,
                status: BiderStatus::Menunggu,
                sudah_menghubungi: false,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.bider.lock().unwrap().push(b.clone());
            Ok(b)
        }

        async fn list_bider_for_iklan(&self, iklan_id: Uuid) -> Result<Vec<Bider>, anyhow::Error> {
            Ok(self
                .bider
                .lock()
                .unwrap()
                .iter()
                .filter(|b| b.iklan_id == iklan_id)
                .cloned()
                .collect())
        }

        async fn has_pending_bider(
            &self,
            iklan_id: Uuid,
            peminat_id: Uuid,
        ) -> Result<bool, anyhow::Error> {
            Ok(self.bider.lock().unwrap().iter().any(|b| {
                b.iklan_id == iklan_id
                    && b.peminat_id == peminat_id
                    && b.status == BiderStatus::Menunggu
            }))
        }

        async fn setujui_bider(
            &self,
            bider_id: Uuid,
            iklan_owner_id: Uuid,
            sudah_menghubungi: bool,
        ) -> Result<Option<Bider>, anyhow::Error> {
            let mut bider = self.bider.lock().unwrap();
            let Some(b) = bider
                .iter_mut()
                .find(|b| b.id == bider_id && b.status == BiderStatus::Menunggu)
            else {
                return Ok(None);
            };
            let is_owner = self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .any(|i| i.id == b.iklan_id && i.seller_id == iklan_owner_id);
            if !is_owner {
                return Ok(None);
            }
            b.status = BiderStatus::Disetujui;
            b.sudah_menghubungi = sudah_menghubungi;
            b.updated_at = chrono::Utc::now();
            let approved = b.clone();
            let iklan_id = approved.iklan_id;
            let approved_id = approved.id;
            drop(bider);

            if let Some(item) = self
                .iklan
                .lock()
                .unwrap()
                .iter_mut()
                .find(|i| i.id == iklan_id)
            {
                item.availability_status = AvailabilityStatus::SudahDiambil;
            }
            for other in self.bider.lock().unwrap().iter_mut() {
                if other.iklan_id == iklan_id
                    && other.id != approved_id
                    && other.status == BiderStatus::Menunggu
                {
                    other.status = BiderStatus::Withdrawn;
                }
            }
            Ok(Some(approved))
        }

        async fn withdraw_bider(
            &self,
            bider_id: Uuid,
            iklan_owner_id: Uuid,
        ) -> Result<Option<Bider>, anyhow::Error> {
            let mut bider = self.bider.lock().unwrap();
            let Some(b) = bider.iter_mut().find(|b| {
                b.id == bider_id
                    && (b.status == BiderStatus::Menunggu || b.status == BiderStatus::Disetujui)
            }) else {
                return Ok(None);
            };
            let is_owner = self
                .iklan
                .lock()
                .unwrap()
                .iter()
                .any(|i| i.id == b.iklan_id && i.seller_id == iklan_owner_id);
            if !is_owner {
                return Ok(None);
            }
            let was_disetujui = b.status == BiderStatus::Disetujui;
            b.status = BiderStatus::Withdrawn;
            b.updated_at = chrono::Utc::now();
            let withdrawn = b.clone();
            let iklan_id = withdrawn.iklan_id;
            drop(bider);

            if was_disetujui {
                if let Some(item) = self
                    .iklan
                    .lock()
                    .unwrap()
                    .iter_mut()
                    .find(|i| i.id == iklan_id)
                {
                    item.availability_status = AvailabilityStatus::Tersedia;
                }
            }
            Ok(Some(withdrawn))
        }

        async fn list_bider_for_peminat(
            &self,
            peminat_id: Uuid,
        ) -> Result<Vec<Bider>, anyhow::Error> {
            Ok(self
                .bider
                .lock()
                .unwrap()
                .iter()
                .filter(|b| b.peminat_id == peminat_id)
                .cloned()
                .collect())
        }
    }

    fn svc() -> IklanBarangBekasService<MockIklanBarangBekasRepository> {
        IklanBarangBekasService::new(std::sync::Arc::new(MockIklanBarangBekasRepository::new()))
    }

    // ── list ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_given_barang_when_list_then_returns_items() {
        let s = svc();
        let seller = Uuid::now_v7();
        s.create(
            seller,
            CreateIklanBarangBekasInput {
                judul: "Laptop".into(),
                deskripsi: "Bekas mulus".into(),
                jenis_barang: "bekas".into(),
                jumlah: 1,
                lokasi_pengambilan: "Jakarta".into(),
                lokasi: None,
                region_id: None,
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
            })
            .await
            .unwrap();
        assert!(!result.is_empty());
        assert_eq!(result[0].judul, "Laptop");
    }

    // ── get ──────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_given_valid_id_when_get_then_returns_barang() {
        let s = svc();
        let seller = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Meja".into(),
                    deskripsi: "Kayu jati".into(),
                    jenis_barang: "baru".into(),
                    jumlah: 2,
                    lokasi_pengambilan: "Bandung".into(),
                    lokasi: Some("Bandung Kota".into()),
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        let got = s.get(created.id).await.unwrap();
        assert_eq!(got.judul, "Meja");
        assert_eq!(got.jumlah, 2);
        assert_eq!(got.jenis_barang, "baru");
    }

    #[tokio::test]
    async fn test_get_given_nonexistent_id_when_get_then_returns_error() {
        let s = svc();
        assert!(s.get(Uuid::now_v7()).await.is_err());
    }

    // ── create ───────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_given_valid_bekas_when_create_then_succeeds() {
        let s = svc();
        let seller = Uuid::now_v7();
        let result = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Buku".into(),
                    deskripsi: "Novel".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 3,
                    lokasi_pengambilan: "Rumah".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.jenis_barang, "bekas");
        assert_eq!(result.jumlah, 3);
        assert_eq!(result.availability_status.as_str(), "tersedia");
    }

    #[tokio::test]
    async fn test_create_given_invalid_jenis_barang_when_create_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let result = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "X".into(),
                    deskripsi: "Y".into(),
                    jenis_barang: "ilegal".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "Z".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("jenis_barang"));
    }

    #[tokio::test]
    async fn test_create_given_html_when_create_then_strips_html() {
        let s = svc();
        let seller = Uuid::now_v7();
        let result = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "<b>Laptop</b>".into(),
                    deskripsi: "<script>xss</script>Desc".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "Lobby <img src=x>".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(!result.judul.contains("<b>"));
        assert!(!result.deskripsi.contains("<script>"));
        assert!(!result.lokasi_pengambilan.contains("<img"));
    }

    // ── mark_taken ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_mark_taken_given_owner_when_mark_taken_then_returns_true() {
        let s = svc();
        let seller = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Ambil".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "Rumah".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(s.mark_taken(created.id, seller).await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_taken_given_other_user_when_mark_taken_then_returns_false() {
        let s = svc();
        let seller = Uuid::now_v7();
        let other = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "NotYours".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "Z".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(!s.mark_taken(created.id, other).await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_taken_idempotent() {
        let s = svc();
        let seller = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Double".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "Any".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(s.mark_taken(created.id, seller).await.unwrap());
        // Kedua kali: idempoten → false (sudah "sudah_diambil")
        assert!(!s.mark_taken(created.id, seller).await.unwrap());
    }

    // ── delete (IDOR) ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_given_owner_when_delete_then_returns_true() {
        let s = svc();
        let seller = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Del".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "X".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(s.delete(created.id, seller).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_given_other_user_when_delete_then_returns_false() {
        let s = svc();
        let seller = Uuid::now_v7();
        let other = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Other".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "X".into(),
                    lokasi: None,
                    region_id: None,
                    foto_urls: None,
                },
            )
            .await
            .unwrap();
        assert!(!s.delete(created.id, other).await.unwrap());
    }

    // ── suspend ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_suspend_given_valid_ids_when_suspend_then_succeeds() {
        let s = svc();
        let seller = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create(
                seller,
                CreateIklanBarangBekasInput {
                    judul: "Sus".into(),
                    deskripsi: "D".into(),
                    jenis_barang: "bekas".into(),
                    jumlah: 1,
                    lokasi_pengambilan: "X".into(),
                    lokasi: None,
                    region_id: None,
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
        assert!(resp.results[0].success);
    }

    // ── entity ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_availability_status_default_tersedia() {
        assert_eq!(AvailabilityStatus::default().as_str(), "tersedia");
    }

    #[test]
    fn test_availability_status_parse_roundtrip() {
        assert_eq!(
            AvailabilityStatus::parse("tersedia"),
            Some(AvailabilityStatus::Tersedia)
        );
        assert_eq!(
            AvailabilityStatus::parse("sudah_diambil"),
            Some(AvailabilityStatus::SudahDiambil)
        );
        assert_eq!(AvailabilityStatus::parse("invalid"), None);
    }

    #[test]
    fn test_availability_status_display() {
        assert_eq!(AvailabilityStatus::Tersedia.to_string(), "tersedia");
        assert_eq!(
            AvailabilityStatus::SudahDiambil.to_string(),
            "sudah_diambil"
        );
    }

    // ── radius filtering (F-1, Kelompok 2 Phase 3) ────────────────────────────────

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

    fn barang_input(lokasi: Option<&str>) -> CreateIklanBarangBekasInput {
        CreateIklanBarangBekasInput {
            judul: "Laptop".into(),
            deskripsi: "Bekas mulus".into(),
            jenis_barang: "bekas".into(),
            jumlah: 1,
            lokasi_pengambilan: "Jakarta".into(),
            lokasi: lokasi.map(String::from),
            region_id: None,
            foto_urls: None,
        }
    }

    /// Filter radius (F-1, PRD §5.14.1): iklan di luar radius default (10km) tidak muncul.
    #[tokio::test]
    async fn test_list_given_lat_lng_when_filtered_then_only_returns_barang_within_radius() {
        let repo = std::sync::Arc::new(MockIklanBarangBekasRepository::new());
        let s = IklanBarangBekasService::new(repo.clone());
        let center = (-6.2, 106.8);
        let near = std::sync::Arc::new(MockGeocodingClient {
            result: Ok(Some(common_geocoding::Coordinates {
                latitude: -6.2 + 5.0 / 111.32,
                longitude: 106.8,
            })),
        });
        let far = std::sync::Arc::new(MockGeocodingClient {
            result: Ok(Some(common_geocoding::Coordinates {
                latitude: -6.2 + 15.0 / 111.32,
                longitude: 106.8,
            })),
        });

        IklanBarangBekasService::new(repo.clone())
            .with_geocoding_client(near)
            .create(Uuid::now_v7(), barang_input(Some("Dekat")))
            .await
            .unwrap();
        IklanBarangBekasService::new(repo.clone())
            .with_geocoding_client(far)
            .create(Uuid::now_v7(), barang_input(Some("Jauh")))
            .await
            .unwrap();

        let result = s
            .list(ListQuery {
                limit: Some(10),
                offset: Some(0),
                latitude: Some(center.0),
                longitude: Some(center.1),
            })
            .await
            .unwrap();

        assert_eq!(
            result.len(),
            1,
            "hanya barang dalam radius yang muncul: {result:?}"
        );
        assert_eq!(result[0].lokasi.as_deref(), Some("Dekat"));
    }

    // ── Bider (F-15, Kelompok 3 Phase 3, PRD §5.14.1-5.14.2 Gambar 5) ────────────

    async fn buat_iklan(
        s: &IklanBarangBekasService<MockIklanBarangBekasRepository>,
        seller: Uuid,
    ) -> Uuid {
        s.create(
            seller,
            CreateIklanBarangBekasInput {
                judul: "Kursi".into(),
                deskripsi: "Masih bagus".into(),
                jenis_barang: "bekas".into(),
                jumlah: 1,
                lokasi_pengambilan: "Rumah".into(),
                lokasi: None,
                region_id: None,
                foto_urls: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn test_ambil_given_own_iklan_when_ambil_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let result = s.ambil(seller, iklan_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("milik sendiri"));
    }

    #[tokio::test]
    async fn test_ambil_given_valid_when_ambil_then_creates_bider_menunggu() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider = s.ambil(peminat, iklan_id).await.unwrap();
        assert_eq!(bider.status.as_str(), "menunggu");
        assert_eq!(bider.peminat_id, peminat);
    }

    #[tokio::test]
    async fn test_ambil_given_pending_bid_exists_when_ambil_again_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        s.ambil(peminat, iklan_id).await.unwrap();
        let result = s.ambil(peminat, iklan_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sudah mengajukan"));
    }

    #[tokio::test]
    async fn test_ambil_given_already_taken_when_ambil_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat_a = Uuid::now_v7();
        let peminat_b = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider_a = s.ambil(peminat_a, iklan_id).await.unwrap();
        s.setujui_bider(
            seller,
            bider_a.id,
            SetujuiBiderInput {
                sudah_menghubungi: true,
            },
        )
        .await
        .unwrap();
        let result = s.ambil(peminat_b, iklan_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("tidak tersedia"));
    }

    #[tokio::test]
    async fn test_list_bider_given_other_user_when_list_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let other = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        s.ambil(peminat, iklan_id).await.unwrap();
        assert!(s.list_bider(other, iklan_id).await.is_err());
    }

    #[tokio::test]
    async fn test_setujui_bider_given_other_user_when_setujui_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let other = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider = s.ambil(peminat, iklan_id).await.unwrap();
        let result = s
            .setujui_bider(
                other,
                bider.id,
                SetujuiBiderInput {
                    sudah_menghubungi: true,
                },
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_withdraw_bider_given_other_user_when_withdraw_then_returns_error() {
        let s = svc();
        let seller = Uuid::now_v7();
        let other = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider = s.ambil(peminat, iklan_id).await.unwrap();
        assert!(s.withdraw_bider(other, bider.id).await.is_err());
    }

    /// P3.4: menyetujui satu bider menandai bider LAIN (masih `menunggu`) sebagai
    /// `withdrawn` ("tidak relevan lagi").
    #[tokio::test]
    async fn test_setujui_bider_given_multiple_pending_when_approved_then_others_withdrawn() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat_a = Uuid::now_v7();
        let peminat_b = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider_a = s.ambil(peminat_a, iklan_id).await.unwrap();
        s.ambil(peminat_b, iklan_id).await.unwrap();

        s.setujui_bider(
            seller,
            bider_a.id,
            SetujuiBiderInput {
                sudah_menghubungi: true,
            },
        )
        .await
        .unwrap();

        let list = s.list_bider(seller, iklan_id).await.unwrap();
        let a = list.iter().find(|b| b.peminat_id == peminat_a).unwrap();
        let b = list.iter().find(|b| b.peminat_id == peminat_b).unwrap();
        assert_eq!(a.status.as_str(), "disetujui");
        assert_eq!(b.status.as_str(), "withdrawn");
    }

    /// P3.5: withdraw bider yang SEBELUMNYA disetujui → iklan otomatis re-listing
    /// (kembali `tersedia`).
    #[tokio::test]
    async fn test_withdraw_bider_given_previously_approved_when_withdrawn_then_iklan_relists() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        let bider = s.ambil(peminat, iklan_id).await.unwrap();
        s.setujui_bider(
            seller,
            bider.id,
            SetujuiBiderInput {
                sudah_menghubungi: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            s.get(iklan_id).await.unwrap().availability_status.as_str(),
            "sudah_diambil"
        );

        s.withdraw_bider(seller, bider.id).await.unwrap();

        assert_eq!(
            s.get(iklan_id).await.unwrap().availability_status.as_str(),
            "tersedia"
        );
    }

    /// P3.6: alur penuh — ambil→withdraw(bider menunggu)→ambil lagi(peminat lain)→setujui.
    /// Withdraw bider yang MASIH `menunggu` (belum pernah disetujui) tidak mengubah status
    /// iklan (tidak pernah delisted) — iklan tetap `tersedia`, jadi peminat lain tetap bisa
    /// mengajukan bid baru ("re-listing" secara fungsional trivial: iklan tidak pernah
    /// hilang dari listing). Lihat test re-listing di atas untuk kasus withdraw bider yang
    /// SUDAH disetujui (transisi status iklan yang sesungguhnya).
    #[tokio::test]
    async fn test_bider_full_flow_ambil_withdraw_ambil_lagi_setujui() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat_a = Uuid::now_v7();
        let peminat_b = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;

        // 1. Ambil (peminat A jadi bider, menunggu).
        let bider_a = s.ambil(peminat_a, iklan_id).await.unwrap();
        assert_eq!(bider_a.status.as_str(), "menunggu");

        // 2. Withdraw (pemilik menolak bider A) — iklan tetap tersedia.
        let withdrawn = s.withdraw_bider(seller, bider_a.id).await.unwrap();
        assert_eq!(withdrawn.status.as_str(), "withdrawn");
        assert_eq!(
            s.get(iklan_id).await.unwrap().availability_status.as_str(),
            "tersedia"
        );

        // 3. Ambil lagi (peminat B, karena iklan masih tersedia).
        let bider_b = s.ambil(peminat_b, iklan_id).await.unwrap();
        assert_eq!(bider_b.status.as_str(), "menunggu");

        // 4. Setujui bider B → iklan SudahDiambil.
        let approved = s
            .setujui_bider(
                seller,
                bider_b.id,
                SetujuiBiderInput {
                    sudah_menghubungi: true,
                },
            )
            .await
            .unwrap();
        assert_eq!(approved.status.as_str(), "disetujui");
        assert_eq!(
            s.get(iklan_id).await.unwrap().availability_status.as_str(),
            "sudah_diambil"
        );
    }

    // ── list_my_ads / list_bider_saya (P4.10/P4.11) ─────────────────────────────

    #[tokio::test]
    async fn test_list_my_ads_given_two_sellers_when_listed_then_only_returns_own() {
        let s = svc();
        let seller_a = Uuid::now_v7();
        let seller_b = Uuid::now_v7();
        buat_iklan(&s, seller_a).await;
        buat_iklan(&s, seller_a).await;
        buat_iklan(&s, seller_b).await;

        let mine = s.list_my_ads(seller_a, 20, 0).await.unwrap();
        assert_eq!(mine.len(), 2);
        assert!(mine.iter().all(|i| i.seller_id == seller_a));
    }

    #[tokio::test]
    async fn test_list_bider_saya_given_bid_when_listed_then_includes_iklan_context() {
        let s = svc();
        let seller = Uuid::now_v7();
        let peminat = Uuid::now_v7();
        let iklan_id = buat_iklan(&s, seller).await;
        s.ambil(peminat, iklan_id).await.unwrap();

        let mine = s.list_bider_saya(peminat).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].iklan_id, iklan_id);
        assert_eq!(mine[0].iklan_judul.as_deref(), Some("Kursi"));
        assert_eq!(mine[0].status.as_str(), "menunggu");
    }
}
