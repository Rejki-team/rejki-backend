use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entity::Report;
use crate::domain::repository::{
    CreateReportParams, ReportListParams, ReportRepository, DEFAULT_LIMIT,
};
use common_rate_limit::RateLimiter;
use report_service_client::ReportStatus;

use super::dto::{ReportDetailResponse, ReportListQuery, ReportResponse};

/// Nama kategori storage — bukan hardcoded string literal.
pub mod storage_category {
    pub const REPORT_EVIDENCE: &str = "report-evidence";
}

pub struct ReportService<R: ReportRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
}

impl<R: ReportRepository> ReportService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    // ── User: create report ────────────────────────────────────────────────

    pub async fn create_report(
        &self,
        reporter_id: Uuid,
        target_type: report_service_client::ReportTargetType,
        target_id: Uuid,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<Report, anyhow::Error> {
        // Rate limit: 10 req/15 menit per user
        if let Some(rl) = &self.rate_limiter {
            if !rl.allow("report:create", &reporter_id.to_string()).await {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        self.repo
            .create(CreateReportParams {
                reporter_id,
                target_type,
                target_id,
                keterangan: &keterangan,
                evidence_object_key: evidence_object_key.as_deref(),
            })
            .await
    }

    // ── Admin: list reports ───────────────────────────────────────────────

    pub async fn admin_list(
        &self,
        query: ReportListQuery,
    ) -> Result<(Vec<ReportResponse>, i64), anyhow::Error> {
        let status = query.status.as_deref().and_then(ReportStatus::parse);
        let result = self
            .repo
            .list(ReportListParams {
                q: query.q,
                status,
                sort_dir: query.sort_dir,
                limit: query.limit.unwrap_or(DEFAULT_LIMIT),
                offset: query.offset.unwrap_or(0),
            })
            .await?;
        Ok((
            result.items.into_iter().map(to_report_resp).collect(),
            result.total,
        ))
    }

    // ── Admin: list all (for CSV) ─────────────────────────────────────────

    pub async fn admin_list_all(
        &self,
        query: ReportListQuery,
    ) -> Result<Vec<ReportResponse>, anyhow::Error> {
        let status = query.status.as_deref().and_then(ReportStatus::parse);
        // Gunakan limit besar untuk CSV; tetap aman karena volume aduan kecil-moderat.
        let result = self
            .repo
            .list(ReportListParams {
                q: query.q,
                status,
                sort_dir: query.sort_dir,
                limit: 10_000,
                offset: 0,
            })
            .await?;
        Ok(result.items.into_iter().map(to_report_resp).collect())
    }

    // ── Admin: get detail ─────────────────────────────────────────────────

    pub async fn admin_get_detail(&self, id: Uuid) -> Result<ReportDetailResponse, anyhow::Error> {
        let report = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))?;
        Ok(to_detail_resp(report))
    }

    // ── Admin: review ─────────────────────────────────────────────────────

    pub async fn admin_review(
        &self,
        id: Uuid,
        approved: bool,
        action_note: String,
        reviewed_by: Uuid,
    ) -> Result<Report, anyhow::Error> {
        let report = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))?;

        if report.status.is_terminal() {
            return Err(anyhow::anyhow!(
                "aduan sudah ditindaklanjuti — tidak dapat diubah"
            ));
        }

        let new_status = if approved {
            ReportStatus::Resolved
        } else {
            ReportStatus::Rejected
        };

        self.repo
            .update_status(id, new_status, &action_note, reviewed_by)
            .await?
            .ok_or_else(|| anyhow::anyhow!("aduan tidak ditemukan"))
    }

    // ── ReportClient in-process ───────────────────────────────────────────

    pub async fn create_report_inproc(
        &self,
        reporter_id: Uuid,
        target_type: report_service_client::ReportTargetType,
        target_id: Uuid,
        keterangan: String,
        evidence_object_key: Option<String>,
    ) -> Result<Uuid, report_service_client::ReportClientError> {
        self.create_report(
            reporter_id,
            target_type,
            target_id,
            keterangan,
            evidence_object_key,
        )
        .await
        .map(|r| r.id)
        .map_err(|_| report_service_client::ReportClientError::Unavailable)
    }

    pub async fn get_status(
        &self,
        report_id: Uuid,
    ) -> Result<ReportStatus, report_service_client::ReportClientError> {
        self.repo
            .find_by_id(report_id)
            .await
            .map_err(|_| report_service_client::ReportClientError::Unavailable)?
            .map(|r| r.status)
            .ok_or(report_service_client::ReportClientError::NotFound)
    }
}

// ── Mapper functions ──────────────────────────────────────────────────────

pub fn to_report_resp(r: Report) -> ReportResponse {
    ReportResponse {
        id: r.id,
        reporter_id: r.reporter_id,
        target_type: r.target_type,
        target_id: r.target_id,
        keterangan: r.keterangan,
        evidence_object_key: r.evidence_object_key,
        status: r.status,
        action_note: r.action_note,
        reviewed_by: r.reviewed_by,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub fn to_detail_resp(r: Report) -> ReportDetailResponse {
    ReportDetailResponse {
        id: r.id,
        reporter_id: r.reporter_id,
        target_type: r.target_type,
        target_id: r.target_id,
        keterangan: r.keterangan,
        evidence_object_key: r.evidence_object_key,
        evidence_read_url: None, // diisi oleh handler setelah request presigned download
        status: r.status,
        action_note: r.action_note,
        reviewed_by: r.reviewed_by,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}
