#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::service::ReportService;
    use crate::domain::entity::Report;
    use crate::domain::repository::{
        CreateReportParams, ListResult, ReportListParams, ReportRepository,
    };
    use report_service_client::{
        ReportAdType, ReportClientError, ReportStatus, ReportTargetType, ReportType,
    };

    // ── MockReportRepository ─────────────────────────────────────────────────────

    struct MockReportRepository {
        reports: Mutex<Vec<Report>>,
    }

    impl MockReportRepository {
        fn new() -> Self {
            Self {
                reports: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl ReportRepository for MockReportRepository {
        async fn create(&self, params: CreateReportParams<'_>) -> Result<Report, anyhow::Error> {
            let report = Report {
                id: Uuid::now_v7(),
                reporter_id: params.reporter_id,
                report_type: params.report_type,
                target_type: params.target_type,
                target_id: params.target_id,
                target_ad_type: params.target_ad_type,
                keterangan: params.keterangan.to_string(),
                evidence_object_key: params.evidence_object_key.map(String::from),
                status: ReportStatus::Pending,
                action_note: None,
                reviewed_by: None,
                due_date: params.due_date,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.reports.lock().unwrap().push(report.clone());
            Ok(report)
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<Report>, anyhow::Error> {
            Ok(self
                .reports
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == id)
                .cloned())
        }

        async fn list(
            &self,
            params: ReportListParams,
        ) -> Result<ListResult<Report>, anyhow::Error> {
            let items: Vec<Report> = self
                .reports
                .lock()
                .unwrap()
                .iter()
                .filter(|r| {
                    params
                        .report_type
                        .map(|rt| rt == r.report_type)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            Ok(ListResult {
                total: items.len() as i64,
                items,
            })
        }

        async fn update_status(
            &self,
            id: Uuid,
            status: ReportStatus,
            action_note: &str,
            reviewed_by: Uuid,
        ) -> Result<Option<Report>, anyhow::Error> {
            let mut reports = self.reports.lock().unwrap();
            if let Some(r) = reports
                .iter_mut()
                .find(|r| r.id == id && !r.status.is_terminal())
            {
                r.status = status;
                r.action_note = Some(action_note.to_string());
                r.reviewed_by = Some(reviewed_by);
                r.updated_at = chrono::Utc::now();
                Ok(Some(r.clone()))
            } else {
                Ok(None)
            }
        }
    }

    fn svc() -> ReportService<MockReportRepository> {
        ReportService::new(std::sync::Arc::new(MockReportRepository::new()))
    }

    // ── Mocks P9.1: admin_approve_and_suspend ────────────────────────────────────

    struct MockAuthClient {
        fail: bool,
        suspend_temp_calls: Mutex<Vec<(Uuid, i64, String)>>,
        suspend_perm_calls: Mutex<Vec<(Uuid, String)>>,
    }

    impl MockAuthClient {
        fn new(fail: bool) -> Self {
            Self {
                fail,
                suspend_temp_calls: Mutex::new(vec![]),
                suspend_perm_calls: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl auth_service_client::AuthClient for MockAuthClient {
        async fn validate_token(
            &self,
            _jwt: &str,
        ) -> Result<auth_service_client::AuthClaims, auth_service_client::AuthClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn get_account_status(
            &self,
            _user_id: Uuid,
        ) -> Result<auth_service_client::AccountStatus, auth_service_client::AuthClientError>
        {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn get_account_email(
            &self,
            _user_id: Uuid,
        ) -> Result<String, auth_service_client::AuthClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn set_account_status(
            &self,
            _user_id: Uuid,
            _status: auth_service_client::AccountStatus,
        ) -> Result<(), auth_service_client::AuthClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn list_active_user_ids(
            &self,
        ) -> Result<Vec<Uuid>, auth_service_client::AuthClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn suspend_temporarily(
            &self,
            user_id: Uuid,
            days: i64,
            reason: &str,
        ) -> Result<(), auth_service_client::AuthClientError> {
            self.suspend_temp_calls
                .lock()
                .unwrap()
                .push((user_id, days, reason.to_string()));
            if self.fail {
                Err(auth_service_client::AuthClientError::Unavailable)
            } else {
                Ok(())
            }
        }
        async fn suspend_permanently(
            &self,
            user_id: Uuid,
            reason: &str,
        ) -> Result<(), auth_service_client::AuthClientError> {
            self.suspend_perm_calls
                .lock()
                .unwrap()
                .push((user_id, reason.to_string()));
            if self.fail {
                Err(auth_service_client::AuthClientError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    struct MockIklanPekerjaanClient {
        suspend_calls: Mutex<Vec<Uuid>>,
    }

    impl MockIklanPekerjaanClient {
        fn new() -> Self {
            Self {
                suspend_calls: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait::async_trait]
    impl iklan_pekerjaan_service_client::IklanPekerjaanClient for MockIklanPekerjaanClient {
        async fn get_summary(
            &self,
            _id: Uuid,
        ) -> Result<
            iklan_pekerjaan_service_client::IklanPekerjaanSummary,
            iklan_pekerjaan_service_client::IklanPekerjaanClientError,
        > {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn exists(
            &self,
            _id: Uuid,
        ) -> Result<bool, iklan_pekerjaan_service_client::IklanPekerjaanClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn is_lamaran_selesai(
            &self,
            _iklan_id: Uuid,
            _poster_id: Uuid,
            _pelamar_id: Uuid,
        ) -> Result<bool, iklan_pekerjaan_service_client::IklanPekerjaanClientError> {
            unimplemented!("tidak dipakai test admin_approve_and_suspend")
        }
        async fn suspend(
            &self,
            iklan_id: Uuid,
            _is_permanent: bool,
            _reason: &str,
            _expires_at: Option<chrono::DateTime<chrono::Utc>>,
            _admin_id: Uuid,
        ) -> Result<(), iklan_pekerjaan_service_client::IklanPekerjaanClientError> {
            self.suspend_calls.lock().unwrap().push(iklan_id);
            Ok(())
        }
    }

    // ── create_laporkan_iklan ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_laporkan_iklan_given_valid_input_when_create_then_succeeds() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target = Uuid::now_v7();
        let report = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                target,
                "Konten tidak pantas sekali dan sangat mengganggu pengguna lain".into(),
            )
            .await
            .unwrap();
        assert_eq!(report.reporter_id, reporter);
        assert_eq!(report.report_type, ReportType::LaporkanIklan);
        assert_eq!(report.target_type, Some(ReportTargetType::Iklan));
        assert_eq!(report.target_id, Some(target));
        assert_eq!(report.status, ReportStatus::Pending);
        assert!(!report.is_overdue());
    }

    #[tokio::test]
    async fn test_create_laporkan_iklan_given_valid_input_when_create_then_due_date_7_business_days_ahead(
    ) {
        let s = svc();
        let report = s
            .create_laporkan_iklan(
                Uuid::now_v7(),
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Konten tidak pantas sekali dan sangat mengganggu pengguna lain".into(),
            )
            .await
            .unwrap();
        // Minimal 7 hari kalender ke depan (7 hari kerja selalu >= 7 hari kalender).
        assert!(report.due_date >= report.created_at + chrono::Duration::days(7));
    }

    // ── create_pelaporan_masalah ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_pelaporan_masalah_given_tanpa_target_when_create_then_succeeds() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let report = s
            .create_pelaporan_masalah(
                reporter,
                None,
                "Proses pembayaran gagal terus menerus".into(),
                "ev-report-001".into(),
            )
            .await
            .unwrap();
        assert_eq!(report.report_type, ReportType::PelaporanMasalah);
        assert_eq!(report.target_type, None);
        assert_eq!(report.target_id, None);
        assert_eq!(report.evidence_object_key.as_deref(), Some("ev-report-001"));
    }

    #[tokio::test]
    async fn test_create_pelaporan_masalah_given_dengan_target_when_create_then_target_type_iklan()
    {
        let s = svc();
        let target = Uuid::now_v7();
        let report = s
            .create_pelaporan_masalah(
                Uuid::now_v7(),
                Some(target),
                "Gagal melamar pekerjaan ini".into(),
                "ev-report-002".into(),
            )
            .await
            .unwrap();
        assert_eq!(report.target_type, Some(ReportTargetType::Iklan));
        assert_eq!(report.target_id, Some(target));
    }

    // ── admin_get_detail ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_get_detail_given_valid_id_when_get_then_returns_report() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let created = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Test keterangan cukup panjang untuk lolos validasi minimal karakter".into(),
            )
            .await
            .unwrap();
        let detail = s.admin_get_detail(created.id).await.unwrap();
        assert_eq!(detail.id, created.id);
        assert_eq!(detail.reporter_id, reporter);
    }

    #[tokio::test]
    async fn test_admin_get_detail_given_nonexistent_id_when_get_then_returns_error() {
        assert!(svc().admin_get_detail(Uuid::now_v7()).await.is_err());
    }

    // ── admin_review ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_review_given_pending_report_when_approve_then_resolved() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Spam berulang kali dikirim oleh pengguna ini ke banyak iklan".into(),
            )
            .await
            .unwrap();
        let reviewed = s
            .admin_review(created.id, true, "Dihapus".into(), admin)
            .await
            .unwrap();
        assert_eq!(reviewed.status, ReportStatus::Resolved);
        assert_eq!(reviewed.action_note, Some("Dihapus".into()));
        assert_eq!(reviewed.reviewed_by, Some(admin));
    }

    #[tokio::test]
    async fn test_admin_review_given_pending_report_when_reject_then_rejected() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Laporan tidak valid karena tidak sesuai dengan kondisi sebenarnya".into(),
            )
            .await
            .unwrap();
        let reviewed = s
            .admin_review(created.id, false, "Tidak valid".into(), admin)
            .await
            .unwrap();
        assert_eq!(reviewed.status, ReportStatus::Rejected);
    }

    #[tokio::test]
    async fn test_admin_review_given_already_reviewed_when_review_again_then_returns_error() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let admin = Uuid::now_v7();
        let created = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Keterangan pengujian ulasan berulang untuk memastikan status terminal".into(),
            )
            .await
            .unwrap();
        s.admin_review(created.id, true, "Resolved".into(), admin)
            .await
            .unwrap();
        let result = s
            .admin_review(created.id, false, "Ubah".into(), admin)
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("sudah ditindaklanjuti"));
    }

    // ── admin_list ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_list_given_reports_when_list_then_returns_all() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target = Uuid::now_v7();
        s.create_laporkan_iklan(
            reporter,
            ReportTargetType::Iklan,
            None,
            target,
            "Laporan pertama dengan keterangan yang cukup panjang untuk validasi".into(),
        )
        .await
        .unwrap();
        s.create_laporkan_iklan(
            reporter,
            ReportTargetType::User,
            None,
            target,
            "Laporan kedua dengan keterangan yang cukup panjang untuk validasi".into(),
        )
        .await
        .unwrap();
        use crate::application::dto::ReportListQuery;
        let (items, total) = s
            .admin_list(ReportListQuery {
                q: None,
                status: None,
                report_type: None,
                sort_dir: None,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_admin_list_given_report_type_filter_when_list_then_returns_only_matching() {
        let s = svc();
        let reporter = Uuid::now_v7();
        s.create_laporkan_iklan(
            reporter,
            ReportTargetType::Iklan,
            None,
            Uuid::now_v7(),
            "Laporkan iklan dengan keterangan yang cukup panjang untuk validasi".into(),
        )
        .await
        .unwrap();
        s.create_pelaporan_masalah(
            reporter,
            None,
            "Pelaporan masalah teknis pada proses".into(),
            "ev-1".into(),
        )
        .await
        .unwrap();

        use crate::application::dto::ReportListQuery;
        let (items, total) = s
            .admin_list(ReportListQuery {
                q: None,
                status: None,
                report_type: Some("pelaporan_masalah".to_string()),
                sort_dir: None,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].report_type, ReportType::PelaporanMasalah);
    }

    // ── ReportClient in-process ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_report_inproc_given_valid_input_when_called_then_returns_uuid() {
        let s = svc();
        let id = s
            .create_report_inproc(
                Uuid::now_v7(),
                ReportTargetType::User,
                Uuid::now_v7(),
                "Test".into(),
                None,
            )
            .await
            .unwrap();
        assert!(!id.is_nil());
    }

    #[tokio::test]
    async fn test_get_status_given_existing_report_when_called_then_returns_status() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let created = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                None,
                Uuid::now_v7(),
                "Keterangan pengujian status laporan dengan panjang yang memadai".into(),
            )
            .await
            .unwrap();
        let status = s.get_status(created.id).await.unwrap();
        assert_eq!(status, ReportStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_status_given_nonexistent_id_when_called_then_returns_not_found() {
        let result = svc().get_status(Uuid::now_v7()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReportClientError::NotFound));
    }

    // ── ReportStatus ────────────────────────────────────────────────────────────

    #[test]
    fn test_report_status_terminal() {
        assert!(!ReportStatus::Pending.is_terminal());
        assert!(!ReportStatus::InReview.is_terminal());
        assert!(ReportStatus::Resolved.is_terminal());
        assert!(ReportStatus::Rejected.is_terminal());
    }

    #[test]
    fn test_report_status_parse_roundtrip() {
        for (s, expected) in [
            ("pending", ReportStatus::Pending),
            ("in_review", ReportStatus::InReview),
            ("resolved", ReportStatus::Resolved),
            ("rejected", ReportStatus::Rejected),
        ] {
            assert_eq!(ReportStatus::parse(s), Some(expected));
            assert_eq!(expected.as_str(), s);
        }
        assert_eq!(ReportStatus::parse("invalid"), None);
    }

    // ── ReportTargetType ────────────────────────────────────────────────────────

    #[test]
    fn test_report_target_type_parse_roundtrip() {
        for (s, expected) in [
            ("iklan", ReportTargetType::Iklan),
            ("user", ReportTargetType::User),
        ] {
            assert_eq!(ReportTargetType::parse(s), Some(expected));
            assert_eq!(expected.as_str(), s);
        }
    }

    // ── ReportType ──────────────────────────────────────────────────────────────

    #[test]
    fn test_report_type_parse_roundtrip() {
        for (s, expected) in [
            ("laporkan_iklan", ReportType::LaporkanIklan),
            ("pelaporan_masalah", ReportType::PelaporanMasalah),
        ] {
            assert_eq!(ReportType::parse(s), Some(expected));
            assert_eq!(expected.as_str(), s);
        }
        assert_eq!(ReportType::parse("invalid"), None);
    }

    // ── is_overdue ──────────────────────────────────────────────────────────────

    #[test]
    fn test_is_overdue_given_due_date_lewat_dan_belum_terminal_then_true() {
        let report = Report {
            id: Uuid::now_v7(),
            reporter_id: Uuid::now_v7(),
            report_type: ReportType::LaporkanIklan,
            target_type: Some(ReportTargetType::Iklan),
            target_id: Some(Uuid::now_v7()),
            target_ad_type: None,
            keterangan: "x".into(),
            evidence_object_key: None,
            status: ReportStatus::Pending,
            action_note: None,
            reviewed_by: None,
            due_date: chrono::Utc::now() - chrono::Duration::days(1),
            created_at: chrono::Utc::now() - chrono::Duration::days(8),
            updated_at: chrono::Utc::now(),
        };
        assert!(report.is_overdue());
    }

    #[test]
    fn test_is_overdue_given_due_date_lewat_tapi_resolved_then_false() {
        let report = Report {
            id: Uuid::now_v7(),
            reporter_id: Uuid::now_v7(),
            report_type: ReportType::LaporkanIklan,
            target_type: Some(ReportTargetType::Iklan),
            target_id: Some(Uuid::now_v7()),
            target_ad_type: None,
            keterangan: "x".into(),
            evidence_object_key: None,
            status: ReportStatus::Resolved,
            action_note: Some("selesai".into()),
            reviewed_by: Some(Uuid::now_v7()),
            due_date: chrono::Utc::now() - chrono::Duration::days(1),
            created_at: chrono::Utc::now() - chrono::Duration::days(8),
            updated_at: chrono::Utc::now(),
        };
        assert!(!report.is_overdue());
    }

    // ── admin_approve_and_suspend (P9.1/P9.3, Kelompok 6 Q9) ─────────────────────

    #[tokio::test]
    async fn test_admin_approve_and_suspend_given_target_user_permanent_then_succeeds_and_resolves()
    {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target_user = Uuid::now_v7();
        let report = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::User,
                None,
                target_user,
                "Aduan pengguna dengan keterangan yang cukup panjang untuk validasi".into(),
            )
            .await
            .unwrap();

        let auth = MockAuthClient::new(false);
        let admin_id = Uuid::now_v7();
        let input = crate::application::dto::ApproveAndSuspendInput {
            reason: "Terbukti melanggar kebijakan platform".into(),
            is_permanent: true,
            expires_at: None,
        };

        let result = s
            .admin_approve_and_suspend(report.id, input, admin_id, &auth, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.status, ReportStatus::Resolved);
        let calls = auth.suspend_perm_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, target_user);
    }

    #[tokio::test]
    async fn test_admin_approve_and_suspend_given_target_iklan_pekerjaan_then_calls_pekerjaan_client(
    ) {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target_iklan = Uuid::now_v7();
        let report = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::Iklan,
                Some(ReportAdType::Pekerjaan),
                target_iklan,
                "Iklan lowongan palsu dan menyesatkan calon pelamar kerja".into(),
            )
            .await
            .unwrap();

        let auth = MockAuthClient::new(false);
        let pekerjaan_client = MockIklanPekerjaanClient::new();
        let admin_id = Uuid::now_v7();
        let input = crate::application::dto::ApproveAndSuspendInput {
            reason: "Terbukti lowongan palsu".into(),
            is_permanent: true,
            expires_at: None,
        };

        let result = s
            .admin_approve_and_suspend(
                report.id,
                input,
                admin_id,
                &auth,
                Some(&pekerjaan_client),
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.status, ReportStatus::Resolved);
        assert_eq!(
            pekerjaan_client.suspend_calls.lock().unwrap().as_slice(),
            &[target_iklan]
        );
        // User-only client tidak boleh terpanggil sama sekali untuk target Iklan.
        assert!(auth.suspend_perm_calls.lock().unwrap().is_empty());
        assert!(auth.suspend_temp_calls.lock().unwrap().is_empty());
    }

    /// Hazard #4: bila suspend GAGAL, status Report TIDAK BOLEH berubah jadi
    /// Resolved — mencegah "aduan sudah Diterima tapi target belum ter-suspend".
    #[tokio::test]
    async fn test_admin_approve_and_suspend_given_suspend_fails_then_report_status_unchanged() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target_user = Uuid::now_v7();
        let report = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::User,
                None,
                target_user,
                "Aduan pengguna dengan keterangan yang cukup panjang untuk validasi".into(),
            )
            .await
            .unwrap();

        let auth = MockAuthClient::new(true); // simulasikan auth-service gagal/down
        let admin_id = Uuid::now_v7();
        let input = crate::application::dto::ApproveAndSuspendInput {
            reason: "Terbukti melanggar kebijakan platform".into(),
            is_permanent: true,
            expires_at: None,
        };

        let result = s
            .admin_approve_and_suspend(report.id, input, admin_id, &auth, None, None, None)
            .await;
        assert!(result.is_err());

        let still = s.admin_get_detail(report.id).await.unwrap();
        assert_eq!(
            still.status,
            ReportStatus::Pending,
            "status TIDAK boleh berubah jadi Resolved bila suspend gagal"
        );
    }

    /// Regresi: web (pola `SuspendIklanPayload`/`SuspendPenggunaPayload`
    /// existing) tidak pernah mengirim `expires_at` spesifik untuk suspend
    /// sementara — sebelumnya ini panic (`.unwrap()` di `None`). Harus jatuh
    /// ke durasi default, bukan panic.
    #[tokio::test]
    async fn test_admin_approve_and_suspend_given_temporary_user_without_expires_at_then_uses_default_days(
    ) {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target_user = Uuid::now_v7();
        let report = s
            .create_laporkan_iklan(
                reporter,
                ReportTargetType::User,
                None,
                target_user,
                "Aduan pengguna dengan keterangan yang cukup panjang untuk validasi".into(),
            )
            .await
            .unwrap();

        let auth = MockAuthClient::new(false);
        let admin_id = Uuid::now_v7();
        let input = crate::application::dto::ApproveAndSuspendInput {
            reason: "Terbukti melanggar kebijakan platform sementara".into(),
            is_permanent: false,
            expires_at: None,
        };

        let result = s
            .admin_approve_and_suspend(report.id, input, admin_id, &auth, None, None, None)
            .await
            .unwrap();

        assert_eq!(result.status, ReportStatus::Resolved);
        let calls = auth.suspend_temp_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, target_user);
        assert!(calls[0].1 > 0, "days harus positif, bukan panic/0");
    }
}
