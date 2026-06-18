#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::service::ReportService;
    use crate::domain::entity::Report;
    use crate::domain::repository::{
        CreateReportParams, ListResult, ReportListParams, ReportRepository,
    };
    use report_service_client::{ReportClientError, ReportStatus, ReportTargetType};

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
                target_type: params.target_type,
                target_id: params.target_id,
                keterangan: params.keterangan.to_string(),
                evidence_object_key: params.evidence_object_key.map(String::from),
                status: ReportStatus::Pending,
                action_note: None,
                reviewed_by: None,
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
            _params: ReportListParams,
        ) -> Result<ListResult<Report>, anyhow::Error> {
            let items = self.reports.lock().unwrap().clone();
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

    // ── create_report ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_report_given_valid_input_when_create_then_succeeds() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let target = Uuid::now_v7();
        let report = s
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                target,
                "Konten tidak pantas".into(),
                Some("ev-001".into()),
            )
            .await
            .unwrap();
        assert_eq!(report.reporter_id, reporter);
        assert_eq!(report.target_type, ReportTargetType::Iklan);
        assert_eq!(report.status, ReportStatus::Pending);
        assert!(report.keterangan.contains("tidak pantas"));
    }

    // ── admin_get_detail ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_get_detail_given_valid_id_when_get_then_returns_report() {
        let s = svc();
        let reporter = Uuid::now_v7();
        let created = s
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                Uuid::now_v7(),
                "Test".into(),
                None,
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
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                Uuid::now_v7(),
                "Spam".into(),
                None,
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
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                Uuid::now_v7(),
                "Invalid".into(),
                None,
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
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                Uuid::now_v7(),
                "X".into(),
                None,
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
        s.create_report(reporter, ReportTargetType::Iklan, target, "A".into(), None)
            .await
            .unwrap();
        s.create_report(reporter, ReportTargetType::User, target, "B".into(), None)
            .await
            .unwrap();
        use crate::application::dto::ReportListQuery;
        let (items, total) = s
            .admin_list(ReportListQuery {
                q: None,
                status: None,
                sort_dir: None,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
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
            .create_report(
                reporter,
                ReportTargetType::Iklan,
                Uuid::now_v7(),
                "Test".into(),
                None,
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
}
