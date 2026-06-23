use std::sync::Arc;

use crate::application::service::ReportService;
use crate::infrastructure::pg_repository::PgReportRepository;
use report_service_client::{ReportClient, ReportClientError, ReportStatus, ReportTargetType};

/// Implementasi ReportClient untuk mode in-process (Modular Monolith).
#[derive(Clone)]
pub struct ReportInProcessClient {
    svc: Arc<ReportService<PgReportRepository>>,
}

impl ReportInProcessClient {
    pub fn new(svc: Arc<ReportService<PgReportRepository>>) -> Self {
        Self { svc }
    }
}

#[allow(async_fn_in_trait)]
impl ReportClient for ReportInProcessClient {
    async fn create_report(
        &self,
        reporter_id: uuid::Uuid,
        target_type: ReportTargetType,
        target_id: uuid::Uuid,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<uuid::Uuid, ReportClientError> {
        self.svc
            .create_report_inproc(
                reporter_id,
                target_type,
                target_id,
                keterangan,
                evidence_object_key,
            )
            .await
    }

    async fn get_report_status(
        &self,
        report_id: uuid::Uuid,
    ) -> Result<ReportStatus, ReportClientError> {
        self.svc.get_status(report_id).await
    }
}
