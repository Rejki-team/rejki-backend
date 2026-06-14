use super::entity::Report;
use report_service_client::{ReportStatus, ReportTargetType};
use uuid::Uuid;

/// Default page size untuk listing.
pub const DEFAULT_LIMIT: i64 = 20;

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreateReportParams<'a> {
    pub reporter_id: Uuid,
    pub target_type: ReportTargetType,
    pub target_id: Uuid,
    pub keterangan: &'a str,
    pub evidence_object_key: Option<&'a str>,
}

/// Params untuk `list()` — search/filter/sort/pagination.
pub struct ReportListParams {
    pub q: Option<String>,
    pub status: Option<ReportStatus>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Hasil listing terpaginasi.
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait ReportRepository: Send + Sync {
    async fn create(&self, params: CreateReportParams<'_>) -> Result<Report, anyhow::Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Report>, anyhow::Error>;
    async fn list(&self, params: ReportListParams) -> Result<ListResult<Report>, anyhow::Error>;
    async fn update_status(
        &self,
        id: Uuid,
        status: ReportStatus,
        action_note: &str,
        reviewed_by: Uuid,
    ) -> Result<Option<Report>, anyhow::Error>;
}
