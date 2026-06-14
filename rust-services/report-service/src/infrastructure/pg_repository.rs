use std::time::Instant;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::Report;
use crate::domain::repository::{
    CreateReportParams, ListResult, ReportListParams, ReportRepository,
};
use report_service_client::{ReportStatus, ReportTargetType};

// Macro expand ke literal string — memenuhi sqlx 0.9 SqlSafeStr (&'static str).
macro_rules! report_cols {
    () => {
        "id,reporter_id,target_type,target_id,keterangan,evidence_object_key,status,action_note,reviewed_by,created_at,updated_at"
    };
}

pub struct PgReportRepository {
    pool: PgPool,
}

impl PgReportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row mapper ────────────────────────────────────────────────────────────

fn row_to_report(r: &sqlx::postgres::PgRow) -> Report {
    Report {
        id: r.get("id"),
        reporter_id: r.get("reporter_id"),
        target_type: ReportTargetType::parse(&r.get::<String, _>("target_type"))
            .unwrap_or(ReportTargetType::Iklan),
        target_id: r.get("target_id"),
        keterangan: r.get("keterangan"),
        evidence_object_key: r.get("evidence_object_key"),
        status: ReportStatus::parse(&r.get::<String, _>("status")).unwrap_or(ReportStatus::Pending),
        action_note: r.get("action_note"),
        reviewed_by: r.get("reviewed_by"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

impl ReportRepository for PgReportRepository {
    async fn create(&self, params: CreateReportParams<'_>) -> Result<Report, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "INSERT INTO report.report (",
            report_cols!(),
            ") VALUES ($1,$2,$3,$4,$5,$6,'pending',NULL,NULL,now(),now()) RETURNING ",
            report_cols!()
        ))
        .bind(Uuid::now_v7())
        .bind(params.reporter_id)
        .bind(params.target_type.as_str())
        .bind(params.target_id)
        .bind(params.keterangan)
        .bind(params.evidence_object_key)
        .fetch_one(&self.pool)
        .await
        .map(|r| row_to_report(&r))?;
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: create report"
            );
        }
        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Report>, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "SELECT ",
            report_cols!(),
            " FROM report.report WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| row_to_report(&r));
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: find_by_id"
            );
        }
        Ok(result)
    }

    async fn list(&self, params: ReportListParams) -> Result<ListResult<Report>, anyhow::Error> {
        let start = Instant::now();

        // Gunakan QueryBuilder untuk query dinamis (sqlx 0.9 safe).
        let mut count_builder =
            sqlx::QueryBuilder::new("SELECT COUNT(*)::bigint FROM report.report WHERE 1=1");
        let mut data_builder = sqlx::QueryBuilder::new(concat!(
            "SELECT ",
            report_cols!(),
            " FROM report.report WHERE 1=1"
        ));

        // Optional: search by ID pengguna atau ID aduan
        if let Some(ref q) = params.q {
            if !q.trim().is_empty() {
                let trimmed = q.trim();
                // Coba parse sebagai UUID untuk pencarian by ID
                if let Ok(uid) = Uuid::parse_str(trimmed) {
                    let uid_str = uid.to_string();
                    count_builder.push(" AND (reporter_id::text = ");
                    count_builder.push_bind(uid_str.clone());
                    count_builder.push(" OR id::text = ");
                    count_builder.push_bind(uid_str.clone());
                    count_builder.push(")");

                    data_builder.push(" AND (reporter_id::text = ");
                    data_builder.push_bind(uid_str.clone());
                    data_builder.push(" OR id::text = ");
                    data_builder.push_bind(uid_str);
                    data_builder.push(")");
                } else {
                    // Fallback: jika bukan UUID, abaikan (tidak ada kolom teks bebas untuk search)
                }
            }
        }

        // Optional: filter by status
        if let Some(ref status) = params.status {
            count_builder.push(" AND status = ");
            count_builder.push_bind(status.as_str());
            data_builder.push(" AND status = ");
            data_builder.push_bind(status.as_str());
        }

        // Total
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await?;

        // Sort
        let sort_dir = params.sort_dir.as_deref().unwrap_or("desc");
        if sort_dir.eq_ignore_ascii_case("asc") {
            data_builder.push(" ORDER BY created_at ASC");
        } else {
            data_builder.push(" ORDER BY created_at DESC");
        }

        // Pagination
        data_builder.push(" LIMIT ");
        data_builder.push_bind(params.limit);
        data_builder.push(" OFFSET ");
        data_builder.push_bind(params.offset);

        let items: Vec<Report> = data_builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(row_to_report)
            .collect();

        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: list reports"
            );
        }
        Ok(ListResult { items, total })
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: ReportStatus,
        action_note: &str,
        reviewed_by: Uuid,
    ) -> Result<Option<Report>, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "UPDATE report.report SET status=$1, action_note=$2, reviewed_by=$3, updated_at=now() ",
            "WHERE id=$4 AND status NOT IN ('resolved','rejected') RETURNING ",
            report_cols!()
        ))
        .bind(status.as_str())
        .bind(action_note)
        .bind(reviewed_by)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| row_to_report(&r));
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: update_status"
            );
        }
        Ok(result)
    }
}
