#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{
        AdminListQuery, CreateIklanBarangBekasInput, ListQuery, SuspendInput,
    };
    use crate::application::service::IklanBarangBekasService;
    use crate::domain::entity::{
        AvailabilityStatus, IklanBarangBekas, IklanSuspension, ModerationStatus,
    };
    use crate::domain::repository::{
        AdminListParams, AdminListResult, CreateBarangBekasParams, IklanBarangBekasRepository,
    };

    // ── MockIklanBarangBekasRepository ────────────────────────────────────────────

    struct MockIklanBarangBekasRepository {
        iklan: Mutex<Vec<IklanBarangBekas>>,
    }

    impl MockIklanBarangBekasRepository {
        fn new() -> Self {
            Self {
                iklan: Mutex::new(vec![]),
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
        ) -> Result<Vec<IklanBarangBekas>, anyhow::Error> {
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
}
