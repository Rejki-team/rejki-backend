#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{
        CreateIklanPelatihanInput, ListQuery, ReviewPelatihanInput, UpdateIklanPelatihanInput,
    };
    use crate::application::service::IklanPelatihanService;
    use crate::domain::entity::{
        CreatedByRole, EnrollmentStatus, IklanPelatihan, IklanSuspension, ModerationStatus,
        PelatihanBadge, PelatihanEnrollment, PelatihanStatus,
    };
    use crate::domain::repository::{
        AdminListParams, AdminListResult, CreatePelatihanParams, IklanPelatihanRepository,
        ListParams, PatchPelatihanParams, UpdatePelatihanParams,
    };

    // ── MockIklanPelatihanRepository ──────────────────────────────────────────────

    struct MockIklanPelatihanRepository {
        pelatihan: Mutex<Vec<IklanPelatihan>>,
        enrollments: Mutex<Vec<PelatihanEnrollment>>,
        badges: Mutex<Vec<PelatihanBadge>>,
    }

    impl MockIklanPelatihanRepository {
        fn new() -> Self {
            Self {
                pelatihan: Mutex::new(vec![]),
                enrollments: Mutex::new(vec![]),
                badges: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl IklanPelatihanRepository for MockIklanPelatihanRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<IklanPelatihan>, anyhow::Error> {
            Ok(self
                .pelatihan
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
        ) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
            Ok(self
                .pelatihan
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
            params: CreatePelatihanParams<'_>,
        ) -> Result<IklanPelatihan, anyhow::Error> {
            let pelatihan = IklanPelatihan {
                id: Uuid::now_v7(),
                poster_id: params.poster_id,
                judul: params.judul.to_string(),
                penyelenggara: params.penyelenggara.to_string(),
                deskripsi: params.deskripsi.to_string(),
                lokasi: params.lokasi.map(String::from),
                region_id: params.region_id.map(String::from),
                harga: params.harga,
                tanggal_mulai: params.tanggal_mulai,
                tanggal_selesai: params.tanggal_selesai,
                foto_urls: vec![],
                is_active: true,
                moderation_status: ModerationStatus::Active,
                status: PelatihanStatus::parse(params.initial_status).unwrap_or_default(),
                created_by_role: CreatedByRole::parse(params.created_by_role).unwrap_or_default(),
                jumlah_peserta: params.jumlah_peserta,
                reviewed_by: None,
                review_note: None,
                deleted_at: None,
                latitude: params.latitude,
                longitude: params.longitude,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                bank_name: Some(params.bank_name.to_string()),
                bank_account_number: Some(params.bank_account_number.to_string()),
                bank_account_holder_name: Some(params.bank_account_holder_name.to_string()),
                signature_object_key: params.signature_object_key.map(String::from),
            };
            self.pelatihan.lock().unwrap().push(pelatihan.clone());
            Ok(pelatihan)
        }

        async fn delete(&self, id: Uuid, poster_id: Uuid) -> Result<bool, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p
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
                .pelatihan
                .lock()
                .unwrap()
                .iter()
                .any(|i| i.id == id && i.deleted_at.is_none()))
        }

        // ── admin moderation ──
        async fn admin_pelatihan_list(
            &self,
            _params: ListParams,
        ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error> {
            let items = self.pelatihan.lock().unwrap().clone();
            Ok(AdminListResult {
                total: items.len() as i64,
                items,
            })
        }
        async fn admin_pelatihan_list_all(
            &self,
            _params: ListParams,
        ) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
            Ok(self.pelatihan.lock().unwrap().clone())
        }
        async fn update_pelatihan(
            &self,
            params: UpdatePelatihanParams<'_>,
        ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p
                .iter_mut()
                .find(|i| i.id == params.id && i.poster_id == params.poster_id)
            {
                item.judul = params.judul.to_string();
                item.penyelenggara = params.penyelenggara.to_string();
                item.deskripsi = params.deskripsi.to_string();
                item.lokasi = params.lokasi.map(String::from);
                item.harga = params.harga;
                item.tanggal_mulai = params.tanggal_mulai;
                item.tanggal_selesai = params.tanggal_selesai;
                item.jumlah_peserta = params.jumlah_peserta;
                item.updated_at = chrono::Utc::now();
                Ok(Some(item.clone()))
            } else {
                Ok(None)
            }
        }
        async fn review_pelatihan(
            &self,
            id: Uuid,
            approved: bool,
            review_note: Option<&str>,
            reviewer_id: Uuid,
        ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p.iter_mut().find(|i| i.id == id && !i.status.is_terminal()) {
                if !item.status.can_review() {
                    return Ok(None);
                }
                item.status = if approved {
                    PelatihanStatus::VerifikasiDiterima
                } else {
                    PelatihanStatus::VerifikasiDitolak
                };
                item.reviewed_by = Some(reviewer_id);
                item.review_note = review_note.map(String::from);
                Ok(Some(item.clone()))
            } else {
                Ok(None)
            }
        }
        async fn soft_delete_pelatihan(
            &self,
            id: Uuid,
            poster_id: Uuid,
        ) -> Result<bool, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p
                .iter_mut()
                .find(|i| i.id == id && i.poster_id == poster_id)
            {
                item.deleted_at = Some(chrono::Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        }

        // ── suspension (existing) ──
        async fn admin_list(
            &self,
            _params: AdminListParams,
        ) -> Result<AdminListResult<IklanPelatihan>, anyhow::Error> {
            let items = self.pelatihan.lock().unwrap().clone();
            Ok(AdminListResult {
                total: items.len() as i64,
                items,
            })
        }
        async fn admin_list_all(
            &self,
            _params: AdminListParams,
        ) -> Result<Vec<IklanPelatihan>, anyhow::Error> {
            Ok(self.pelatihan.lock().unwrap().clone())
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
                if self.pelatihan.lock().unwrap().iter().any(|i| i.id == *id) {
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
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p.iter_mut().find(|i| i.id == id) {
                item.deleted_at = Some(chrono::Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        }
        async fn set_pelatihan_status_system(
            &self,
            id: Uuid,
            new_status: &str,
        ) -> Result<bool, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p
                .iter_mut()
                .find(|i| i.id == id && i.deleted_at.is_none() && !i.status.is_terminal())
            {
                item.status = PelatihanStatus::parse(new_status).unwrap_or(item.status.clone());
                item.updated_at = chrono::Utc::now();
                Ok(true)
            } else {
                Ok(false)
            }
        }
        async fn has_any_badge_for_pelatihan(
            &self,
            pelatihan_id: Uuid,
        ) -> Result<bool, anyhow::Error> {
            Ok(self
                .badges
                .lock()
                .unwrap()
                .iter()
                .any(|b| b.pelatihan_id == pelatihan_id))
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
            poster_id: Uuid,
            _params: PatchPelatihanParams,
        ) -> Result<Option<IklanPelatihan>, anyhow::Error> {
            let mut p = self.pelatihan.lock().unwrap();
            if let Some(item) = p
                .iter_mut()
                .find(|i| i.id == id && i.poster_id == poster_id && i.deleted_at.is_none())
            {
                item.updated_at = chrono::Utc::now();
                Ok(Some(item.clone()))
            } else {
                Ok(None)
            }
        }

        // ── enrollment ──
        async fn create_enrollment(
            &self,
            pelatihan_id: Uuid,
            user_id: Uuid,
        ) -> Result<PelatihanEnrollment, anyhow::Error> {
            let e = PelatihanEnrollment {
                id: Uuid::now_v7(),
                pelatihan_id,
                user_id,
                bukti_transfer_object_key: None,
                status: EnrollmentStatus::Pending,
                reviewed_by: None,
                review_note: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.enrollments.lock().unwrap().push(e.clone());
            Ok(e)
        }
        async fn find_enrollment_by_id(
            &self,
            id: Uuid,
        ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
            Ok(self
                .enrollments
                .lock()
                .unwrap()
                .iter()
                .find(|e| e.id == id)
                .cloned())
        }
        async fn commit_enrollment_bukti(
            &self,
            id: Uuid,
            user_id: Uuid,
            object_key: &str,
        ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
            let mut enrollments = self.enrollments.lock().unwrap();
            if let Some(e) = enrollments.iter_mut().find(|e| {
                e.id == id && e.user_id == user_id && e.bukti_transfer_object_key.is_none()
            }) {
                e.bukti_transfer_object_key = Some(object_key.to_string());
                e.status = EnrollmentStatus::InReview;
                Ok(Some(e.clone()))
            } else {
                Ok(None)
            }
        }
        async fn admin_enrollment_list(
            &self,
            _params: ListParams,
        ) -> Result<AdminListResult<PelatihanEnrollment>, anyhow::Error> {
            let items = self.enrollments.lock().unwrap().clone();
            Ok(AdminListResult {
                total: items.len() as i64,
                items,
            })
        }
        async fn admin_enrollment_list_all(
            &self,
            _params: ListParams,
        ) -> Result<Vec<PelatihanEnrollment>, anyhow::Error> {
            Ok(self.enrollments.lock().unwrap().clone())
        }
        async fn review_enrollment(
            &self,
            id: Uuid,
            approved: bool,
            review_note: Option<&str>,
            reviewer_id: Uuid,
        ) -> Result<Option<PelatihanEnrollment>, anyhow::Error> {
            let mut enrollments = self.enrollments.lock().unwrap();
            if let Some(e) = enrollments
                .iter_mut()
                .find(|e| e.id == id && e.status.can_review())
            {
                e.status = if approved {
                    EnrollmentStatus::Approved
                } else {
                    EnrollmentStatus::Rejected
                };
                e.reviewed_by = Some(reviewer_id);
                e.review_note = review_note.map(String::from);
                Ok(Some(e.clone()))
            } else {
                Ok(None)
            }
        }

        // ── badge ──
        async fn create_badge(
            &self,
            pelatihan_id: Uuid,
            user_id: Uuid,
        ) -> Result<PelatihanBadge, anyhow::Error> {
            let b = PelatihanBadge {
                id: Uuid::now_v7(),
                pelatihan_id,
                user_id,
                sertifikat_object_key: None,
                approved_at: None,
                status: EnrollmentStatus::Pending,
                reviewed_by: None,
                review_note: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.badges.lock().unwrap().push(b.clone());
            Ok(b)
        }
        async fn find_badge_by_id(
            &self,
            id: Uuid,
        ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
            Ok(self
                .badges
                .lock()
                .unwrap()
                .iter()
                .find(|b| b.id == id)
                .cloned())
        }
        async fn commit_badge_sertifikat(
            &self,
            id: Uuid,
            user_id: Uuid,
            object_key: &str,
        ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
            let mut badges = self.badges.lock().unwrap();
            if let Some(b) = badges
                .iter_mut()
                .find(|b| b.id == id && b.user_id == user_id && b.sertifikat_object_key.is_none())
            {
                b.sertifikat_object_key = Some(object_key.to_string());
                b.status = EnrollmentStatus::InReview;
                Ok(Some(b.clone()))
            } else {
                Ok(None)
            }
        }
        async fn set_badge_sertifikat(
            &self,
            id: Uuid,
            object_key: &str,
        ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
            let mut badges = self.badges.lock().unwrap();
            if let Some(b) = badges.iter_mut().find(|b| b.id == id) {
                b.sertifikat_object_key = Some(object_key.to_string());
                Ok(Some(b.clone()))
            } else {
                Ok(None)
            }
        }
        async fn admin_badge_list(
            &self,
            _params: ListParams,
        ) -> Result<AdminListResult<PelatihanBadge>, anyhow::Error> {
            let items = self.badges.lock().unwrap().clone();
            Ok(AdminListResult {
                total: items.len() as i64,
                items,
            })
        }
        async fn admin_badge_list_all(
            &self,
            _params: ListParams,
        ) -> Result<Vec<PelatihanBadge>, anyhow::Error> {
            Ok(self.badges.lock().unwrap().clone())
        }
        async fn review_badge(
            &self,
            id: Uuid,
            approved: bool,
            review_note: Option<&str>,
            reviewer_id: Uuid,
        ) -> Result<Option<PelatihanBadge>, anyhow::Error> {
            let mut badges = self.badges.lock().unwrap();
            if let Some(b) = badges
                .iter_mut()
                .find(|b| b.id == id && b.status.can_review())
            {
                b.status = if approved {
                    EnrollmentStatus::Approved
                } else {
                    EnrollmentStatus::Rejected
                };
                if approved {
                    b.approved_at = Some(chrono::Utc::now());
                }
                b.reviewed_by = Some(reviewer_id);
                b.review_note = review_note.map(String::from);
                Ok(Some(b.clone()))
            } else {
                Ok(None)
            }
        }
    }

    fn svc() -> IklanPelatihanService<MockIklanPelatihanRepository> {
        IklanPelatihanService::new(std::sync::Arc::new(MockIklanPelatihanRepository::new()))
    }

    fn basic_input() -> CreateIklanPelatihanInput {
        CreateIklanPelatihanInput {
            judul: "Rust Bootcamp".into(),
            penyelenggara: "PT Edu".into(),
            deskripsi: "Belajar Rust".into(),
            lokasi: None,
            region_id: None,
            harga: Some(500_000),
            tanggal_mulai: None,
            tanggal_selesai: None,
            jumlah_peserta: Some(30),
            foto_urls: None,
            bank_name: "BCA".into(),
            bank_account_number: "1234567890".into(),
            bank_account_holder_name: "PT Edu".into(),
            signature_object_key: None,
        }
    }

    // ── list ─────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_given_pelatihan_when_list_then_returns_items() {
        let s = svc();
        let poster = Uuid::now_v7();
        s.create_user(poster, basic_input()).await.unwrap();
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
        // ammonia encodes spaces as &#32; — verify the title content survived
        assert!(result[0].judul.contains("Rust"));
        assert!(result[0].judul.contains("Bootcamp"));
    }

    // ── get ──────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_given_valid_id_when_get_then_returns_pelatihan() {
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s
            .create_user(
                poster,
                CreateIklanPelatihanInput {
                    judul: "Workshop".into(),
                    penyelenggara: "Org".into(),
                    deskripsi: "D".into(),
                    lokasi: Some("Online".into()),
                    region_id: None,
                    harga: Some(100_000),
                    tanggal_mulai: None,
                    tanggal_selesai: None,
                    jumlah_peserta: Some(50),
                    foto_urls: None,
                    bank_name: "BCA".into(),
                    bank_account_number: "1234567890".into(),
                    bank_account_holder_name: "Org".into(),
                    signature_object_key: None,
                },
            )
            .await
            .unwrap();
        let got = s.get(created.id).await.unwrap();
        assert_eq!(got.judul, "Workshop");
        assert_eq!(got.penyelenggara, "Org");
    }

    #[tokio::test]
    async fn test_get_given_nonexistent_id_when_get_then_returns_error() {
        assert!(svc().get(Uuid::now_v7()).await.is_err());
    }

    // ── create_user vs create_admin ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_user_given_valid_input_when_create_then_status_verifikasi_tertunda() {
        let s = svc();
        let poster = Uuid::now_v7();
        let result = s.create_user(poster, basic_input()).await.unwrap();
        assert_eq!(result.status.as_str(), "verifikasi_tertunda");
        assert_eq!(result.created_by_role.as_str(), "user");
    }

    #[tokio::test]
    async fn test_create_admin_given_valid_input_when_create_then_status_verifikasi_diterima() {
        let s = svc();
        let admin = Uuid::now_v7();
        let result = s.create_admin(admin, basic_input()).await.unwrap();
        assert_eq!(result.status.as_str(), "verifikasi_diterima");
        assert_eq!(result.created_by_role.as_str(), "admin");
    }

    #[tokio::test]
    async fn test_create_user_given_html_when_create_then_strips_html() {
        let s = svc();
        let poster = Uuid::now_v7();
        let result = s
            .create_user(
                poster,
                CreateIklanPelatihanInput {
                    judul: "<b>Bootcamp</b>".into(),
                    penyelenggara: "<script>evil</script>Edu".into(),
                    deskripsi: "<a href='xss'>Link</a>".into(),
                    lokasi: None,
                    region_id: None,
                    harga: None,
                    tanggal_mulai: None,
                    tanggal_selesai: None,
                    jumlah_peserta: None,
                    foto_urls: None,
                    bank_name: "BCA".into(),
                    bank_account_number: "1234567890".into(),
                    bank_account_holder_name: "Edu".into(),
                    signature_object_key: None,
                },
            )
            .await
            .unwrap();
        assert!(!result.judul.contains("<b>"));
        assert!(!result.penyelenggara.contains("<script>"));
    }

    // ── delete ───────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_delete_given_owner_when_delete_then_returns_true() {
        let s = svc();
        let poster = Uuid::now_v7();
        let created = s.create_user(poster, basic_input()).await.unwrap();
        assert!(s.delete(created.id, poster).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_given_other_user_when_delete_then_returns_false() {
        let s = svc();
        let poster = Uuid::now_v7();
        let other = Uuid::now_v7();
        let created = s.create_user(poster, basic_input()).await.unwrap();
        assert!(!s.delete(created.id, other).await.unwrap());
    }

    // ── admin update ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_update_pelatihan_given_admin_owner_when_update_then_succeeds() {
        let s = svc();
        let admin = Uuid::now_v7();
        let created = s.create_admin(admin, basic_input()).await.unwrap();
        let updated = s
            .admin_update_pelatihan(
                admin,
                created.id,
                UpdateIklanPelatihanInput {
                    judul: "Updated Bootcamp".into(),
                    penyelenggara: "Updated Edu".into(),
                    deskripsi: "Updated desc".into(),
                    lokasi: Some("Bandung".into()),
                    region_id: None,
                    harga: Some(750_000),
                    tanggal_mulai: None,
                    tanggal_selesai: None,
                    jumlah_peserta: Some(40),
                    bank_name: "BCA".into(),
                    bank_account_number: "1234567890".into(),
                    bank_account_holder_name: "Updated Edu".into(),
                    signature_object_key: None,
                },
            )
            .await
            .unwrap();
        assert!(updated.judul.contains("Updated"));
        assert!(updated.penyelenggara.contains("Updated"));
        assert!(updated.penyelenggara.contains("Edu"));
        assert_eq!(updated.jumlah_peserta, Some(40));
    }

    #[tokio::test]
    async fn test_admin_update_pelatihan_given_user_created_when_update_then_returns_error() {
        let s = svc();
        let poster = Uuid::now_v7();
        let admin = poster; // admin acting on user's content
        let created = s.create_user(poster, basic_input()).await.unwrap();
        // User-created pelatihan tidak bisa diedit oleh admin (poster_id sama tapi created_by_role=User)
        let result = s
            .admin_update_pelatihan(
                admin,
                created.id,
                UpdateIklanPelatihanInput {
                    judul: "Hack".into(),
                    penyelenggara: "X".into(),
                    deskripsi: "Y".into(),
                    lokasi: None,
                    region_id: None,
                    harga: None,
                    tanggal_mulai: None,
                    tanggal_selesai: None,
                    jumlah_peserta: None,
                    bank_name: "BCA".into(),
                    bank_account_number: "1234567890".into(),
                    bank_account_holder_name: "X".into(),
                    signature_object_key: None,
                },
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("tidak dapat disunting"));
    }

    // ── admin review pelatihan ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_review_pelatihan_given_user_content_when_approve_then_verifikasi_diterima()
    {
        let s = svc();
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s.create_user(poster, basic_input()).await.unwrap();
        let reviewed = s
            .admin_review_pelatihan(
                admin,
                created.id,
                ReviewPelatihanInput {
                    approved: true,
                    review_note: Some("OK".into()),
                },
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(reviewed.status.as_str(), "verifikasi_diterima");
    }

    #[tokio::test]
    async fn test_admin_review_pelatihan_given_user_content_when_reject_without_note_then_returns_error(
    ) {
        let s = svc();
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s.create_user(poster, basic_input()).await.unwrap();
        let result = s
            .admin_review_pelatihan(
                admin,
                created.id,
                ReviewPelatihanInput {
                    approved: false,
                    review_note: None,
                },
                None,
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Alasan penolakan"));
    }

    #[tokio::test]
    async fn test_admin_review_pelatihan_given_admin_content_when_review_then_returns_error() {
        let s = svc();
        let admin = Uuid::now_v7();
        let created = s.create_admin(admin, basic_input()).await.unwrap();
        let result = s
            .admin_review_pelatihan(
                admin,
                created.id,
                ReviewPelatihanInput {
                    approved: true,
                    review_note: Some("OK".into()),
                },
                None,
                None,
            )
            .await;
        assert!(result.is_err());
        // Admin-created pelatihan → status VerifikasiDiterima → can_review()=false
        // Either error: "sudah diverifikasi sebelumnya" or "tidak memerlukan review"
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sudah diverifikasi") || err.contains("tidak memerlukan review"));
    }

    #[tokio::test]
    async fn test_admin_review_pelatihan_given_already_reviewed_when_review_again_then_returns_error(
    ) {
        let s = svc();
        let poster = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s.create_user(poster, basic_input()).await.unwrap();
        // approve pertama
        s.admin_review_pelatihan(
            admin,
            created.id,
            ReviewPelatihanInput {
                approved: true,
                review_note: Some("OK".into()),
            },
            None,
            None,
        )
        .await
        .unwrap();
        // approve kedua → error
        let result = s
            .admin_review_pelatihan(
                admin,
                created.id,
                ReviewPelatihanInput {
                    approved: false,
                    review_note: Some("Reconsider".into()),
                },
                None,
                None,
            )
            .await;
        assert!(result.is_err());
    }

    // ── enroll ───────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_enrollment_given_verified_pelatihan_when_enroll_then_succeeds() {
        let s = svc();
        let admin = Uuid::now_v7();
        let user = Uuid::now_v7();
        let pelatihan = s.create_admin(admin, basic_input()).await.unwrap(); // auto-approve → verifikasi_diterima
        let enrollment = s.create_enrollment(pelatihan.id, user).await.unwrap();
        assert!(enrollment.status.as_str() == "pending" || enrollment.status.as_str() == "pending");
        assert_eq!(enrollment.user_id, user);
    }

    // ── entity ───────────────────────────────────────────────────────────────────

    #[test]
    fn test_pelatihan_status_terminal() {
        assert!(!PelatihanStatus::VerifikasiTertunda.is_terminal());
        assert!(!PelatihanStatus::VerifikasiDalamProses.is_terminal());
        assert!(PelatihanStatus::VerifikasiDitolak.is_terminal());
        assert!(!PelatihanStatus::VerifikasiDiterima.is_terminal());
        assert!(!PelatihanStatus::PelatihanBelumDimulai.is_terminal());
        assert!(!PelatihanStatus::PelatihanBerjalan.is_terminal());
        assert!(PelatihanStatus::PelatihanSelesai.is_terminal());
    }

    #[test]
    fn test_pelatihan_status_can_review() {
        assert!(PelatihanStatus::VerifikasiTertunda.can_review());
        assert!(PelatihanStatus::VerifikasiDalamProses.can_review());
        assert!(!PelatihanStatus::VerifikasiDiterima.can_review());
    }

    #[test]
    fn test_pelatihan_status_parse_roundtrip() {
        let cases = [
            ("verifikasi_tertunda", PelatihanStatus::VerifikasiTertunda),
            ("pelatihan_selesai", PelatihanStatus::PelatihanSelesai),
        ];
        for (s, ref expected) in cases {
            assert_eq!(PelatihanStatus::parse(s).as_ref(), Some(expected));
            assert_eq!(expected.as_str(), s);
        }
        assert_eq!(PelatihanStatus::parse("invalid"), None);
    }

    #[test]
    fn test_enrollment_status_can_review() {
        assert!(EnrollmentStatus::Pending.can_review());
        assert!(EnrollmentStatus::InReview.can_review());
        assert!(!EnrollmentStatus::Approved.can_review());
        assert!(!EnrollmentStatus::Rejected.can_review());
    }

    // ── radius filtering (F-1, F-14, Kelompok 2 Phase 3) ──────────────────────────

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

    fn training_input(lokasi: Option<&str>) -> CreateIklanPelatihanInput {
        CreateIklanPelatihanInput {
            lokasi: lokasi.map(String::from),
            ..basic_input()
        }
    }

    /// Filter radius (F-1/F-14, PRD §5.13.1): iklan di luar radius default (10km) tidak muncul.
    #[tokio::test]
    async fn test_list_given_lat_lng_when_filtered_then_only_returns_pelatihan_within_radius() {
        let repo = std::sync::Arc::new(MockIklanPelatihanRepository::new());
        let s = IklanPelatihanService::new(repo.clone());
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

        IklanPelatihanService::new(repo.clone())
            .with_geocoding_client(near)
            .create_user(Uuid::now_v7(), training_input(Some("Dekat")))
            .await
            .unwrap();
        IklanPelatihanService::new(repo.clone())
            .with_geocoding_client(far)
            .create_user(Uuid::now_v7(), training_input(Some("Jauh")))
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
            "hanya pelatihan dalam radius yang muncul: {result:?}"
        );
        assert_eq!(result[0].lokasi.as_deref(), Some("Dekat"));
    }

    // ── admin_review_badge: generate sertifikat otomatis (F-12, Kelompok 6 P6.3/P6.5) ──

    /// Mock `CertificateGenerator` — hanya mencatat input yang diterima, tidak
    /// render PDF sungguhan (itu sudah diuji terpisah di
    /// `infrastructure::certificate_generator::tests`). Menjaga test service
    /// ini tetap di boundary application+domain (tidak import infrastructure).
    struct MockCertificateGenerator {
        peserta_nama_calls: Mutex<Vec<String>>,
    }

    impl MockCertificateGenerator {
        fn new() -> Self {
            Self {
                peserta_nama_calls: Mutex::new(vec![]),
            }
        }
    }

    impl crate::domain::cert_generator::CertificateGenerator for MockCertificateGenerator {
        fn generate(
            &self,
            input: crate::domain::cert_generator::CertificateInput,
        ) -> Result<Vec<u8>, anyhow::Error> {
            self.peserta_nama_calls
                .lock()
                .unwrap()
                .push(input.peserta_nama.to_string());
            Ok(b"%PDF-mock-cert".to_vec())
        }
    }

    struct MockStorageClient {
        upload_calls: std::sync::atomic::AtomicUsize,
        download_calls: std::sync::atomic::AtomicUsize,
    }

    impl MockStorageClient {
        fn new() -> Self {
            Self {
                upload_calls: std::sync::atomic::AtomicUsize::new(0),
                download_calls: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl storage_service_client::StorageClient for MockStorageClient {
        async fn request_upload(
            &self,
            _category: &str,
            _user_id: Uuid,
            _info: storage_service_client::FileInfo,
        ) -> Result<
            storage_service_client::UploadPermission,
            storage_service_client::StorageClientError,
        > {
            unimplemented!("tidak dipakai test admin_review_badge")
        }
        async fn request_download(
            &self,
            object_key: &str,
        ) -> Result<String, storage_service_client::StorageClientError> {
            Ok(format!("https://mock-storage.local/{object_key}"))
        }
        async fn delete(
            &self,
            _object_key: &str,
        ) -> Result<(), storage_service_client::StorageClientError> {
            unimplemented!("tidak dipakai test admin_review_badge")
        }
        async fn download_bytes(
            &self,
            _object_key: &str,
        ) -> Result<Vec<u8>, storage_service_client::StorageClientError> {
            self.download_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![1, 2, 3, 4])
        }
        async fn upload_bytes(
            &self,
            _category: &str,
            _user_id: Uuid,
            _bytes: Vec<u8>,
            _mime: &str,
        ) -> Result<String, storage_service_client::StorageClientError> {
            let n = self
                .upload_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(format!("uploads/training-certificate/cert-{n}.pdf"))
        }
    }

    struct MockUserClientForCert;

    #[async_trait::async_trait]
    impl user_service_client::UserClient for MockUserClientForCert {
        async fn get_user_summary(
            &self,
            user_id: Uuid,
        ) -> Result<user_service_client::UserSummary, user_service_client::UserClientError>
        {
            Ok(user_service_client::UserSummary {
                id: user_id,
                username: "Budi Peserta".into(),
                avatar: None,
            })
        }
        async fn user_exists(
            &self,
            _user_id: Uuid,
        ) -> Result<bool, user_service_client::UserClientError> {
            unimplemented!()
        }
        async fn purge_kyc_documents(
            &self,
            _user_id: Uuid,
        ) -> Result<(), user_service_client::UserClientError> {
            unimplemented!()
        }
        async fn get_sensitive_doc_flags(
            &self,
            _auth_id: Uuid,
        ) -> Result<user_service_client::SensitiveDocFlags, user_service_client::UserClientError>
        {
            unimplemented!()
        }
        async fn admin_reveal_nik(
            &self,
            _auth_id: Uuid,
            _admin_id: Uuid,
        ) -> Result<Option<String>, user_service_client::UserClientError> {
            unimplemented!()
        }
        async fn admin_get_document_url(
            &self,
            _auth_id: Uuid,
            _kind: &str,
            _admin_id: Uuid,
        ) -> Result<Option<String>, user_service_client::UserClientError> {
            unimplemented!()
        }
        async fn get_location_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<
            Vec<user_service_client::UserLocationSummary>,
            user_service_client::UserClientError,
        > {
            unimplemented!()
        }
        async fn get_demographic_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<
            Vec<user_service_client::UserDemographicSummary>,
            user_service_client::UserClientError,
        > {
            unimplemented!()
        }
        async fn get_summaries_by_auth_ids(
            &self,
            _auth_ids: &[Uuid],
        ) -> Result<Vec<user_service_client::UserSummary>, user_service_client::UserClientError>
        {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_admin_review_badge_given_approved_when_review_then_generates_certificate_once() {
        let s = svc();
        let admin = Uuid::now_v7();
        let peserta = Uuid::now_v7();

        let mut input = basic_input();
        input.lokasi = Some("Jl. Merdeka No. 1, Jakarta".into());
        input.bank_account_holder_name = "Siti Aminah".into();
        input.signature_object_key = Some("sig-object-key".into());
        let pelatihan = s.create_admin(admin, input).await.unwrap();
        let badge = s.create_badge(pelatihan.id, peserta).await.unwrap();

        let storage = MockStorageClient::new();
        let user_client = MockUserClientForCert;
        let cert_gen = MockCertificateGenerator::new();

        let result = s
            .admin_review_badge(
                admin,
                badge.id,
                crate::application::dto::ReviewBadgeInput {
                    approved: true,
                    review_note: None,
                },
                None,
                None,
                Some(&storage),
                Some(&user_client),
                Some(&cert_gen),
            )
            .await
            .unwrap();

        assert!(result.sertifikat_object_key.is_some());
        assert_eq!(
            storage
                .upload_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert_eq!(
            storage
                .download_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        let calls = cert_gen.peserta_nama_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "Budi Peserta");
    }

    // P6.5: N peserta (badge terpisah) direview satu per satu → setiap review
    // menghasilkan TEPAT SATU panggilan upload (hindari N+1 akibat state
    // yang tidak sengaja terbagi antar pemanggilan berurutan).
    #[tokio::test]
    async fn test_admin_review_badge_given_n_badges_when_review_each_then_one_upload_per_badge() {
        let s = svc();
        let admin = Uuid::now_v7();
        let pelatihan = s.create_admin(admin, basic_input()).await.unwrap();

        let storage = MockStorageClient::new();
        let user_client = MockUserClientForCert;
        let cert_gen = MockCertificateGenerator::new();

        const N: usize = 3;
        let mut sertifikat_keys = Vec::new();
        for _ in 0..N {
            let peserta = Uuid::now_v7();
            let badge = s.create_badge(pelatihan.id, peserta).await.unwrap();
            let result = s
                .admin_review_badge(
                    admin,
                    badge.id,
                    crate::application::dto::ReviewBadgeInput {
                        approved: true,
                        review_note: None,
                    },
                    None,
                    None,
                    Some(&storage),
                    Some(&user_client),
                    Some(&cert_gen),
                )
                .await
                .unwrap();
            sertifikat_keys.push(result.sertifikat_object_key.unwrap());
        }

        assert_eq!(
            storage
                .upload_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            N
        );
        let unique: std::collections::HashSet<_> = sertifikat_keys.iter().collect();
        assert_eq!(unique.len(), N, "setiap badge dapat object_key sendiri");
    }
}
