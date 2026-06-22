use anyhow::Context;
use sqlx::PgPool;

use crate::domain::entity::{EngagementStats, GeoStats, IklanStats, UserStats};
use crate::domain::repository::InsightsRepository;

pub struct PgInsightsRepository {
    pool: PgPool,
}

impl PgInsightsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Helper untuk slow query warn.
macro_rules! warn_slow {
    ($start:expr, $op:expr) => {
        let elapsed = $start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::warn!(op = $op, elapsed_ms = elapsed.as_millis(), "slow DB query");
        }
    };
}

impl InsightsRepository for PgInsightsRepository {
    async fn get_user_stats(&self) -> Result<UserStats, anyhow::Error> {
        let start = std::time::Instant::now();
        let row: UserStats = sqlx::query_as(
            "SELECT \
                COALESCE(total_users, 0)::bigint AS total_users, \
                COALESCE(new_users_today, 0)::bigint AS new_users_today, \
                COALESCE(new_users_week, 0)::bigint AS new_users_week, \
                COALESCE(new_users_month, 0)::bigint AS new_users_month, \
                COALESCE(active_users, 0)::bigint AS active_users, \
                COALESCE(verified_users, 0)::bigint AS verified_users, \
                COALESCE(prev_month_users, 0)::bigint AS prev_month_users, \
                COALESCE(funnel_registered, 0)::bigint AS funnel_registered, \
                COALESCE(funnel_profile_filled, 0)::bigint AS funnel_profile_filled, \
                COALESCE(funnel_kyc_submitted, 0)::bigint AS funnel_kyc_submitted, \
                COALESCE(funnel_kyc_approved, 0)::bigint AS funnel_kyc_approved \
            FROM analytics.mv_user_stats",
        )
        .fetch_one(&self.pool)
        .await
        .context("gagal query mv_user_stats")?;
        warn_slow!(start, "mv_user_stats");
        Ok(row)
    }

    async fn get_iklan_stats(&self) -> Result<Vec<IklanStats>, anyhow::Error> {
        let start = std::time::Instant::now();
        let rows: Vec<IklanStats> = sqlx::query_as(
            "SELECT \
                vertikal, \
                COALESCE(aktif, 0)::bigint AS aktif, \
                COALESCE(total, 0)::bigint AS total, \
                COALESCE(baru_7d, 0)::bigint AS baru_7d, \
                COALESCE(baru_30d, 0)::bigint AS baru_30d, \
                COALESCE(terjual, 0)::bigint AS terjual, \
                COALESCE(suspended, 0)::bigint AS suspended \
            FROM analytics.mv_iklan_stats \
            ORDER BY vertikal",
        )
        .fetch_all(&self.pool)
        .await
        .context("gagal query mv_iklan_stats")?;
        warn_slow!(start, "mv_iklan_stats");
        Ok(rows)
    }

    async fn get_geo_stats(
        &self,
        province_id: Option<&str>,
    ) -> Result<Vec<GeoStats>, anyhow::Error> {
        let start = std::time::Instant::now();
        let rows: Vec<GeoStats> = if let Some(pid) = province_id {
            sqlx::query_as(
                "SELECT \
                    province_id, \
                    province_name, \
                    COALESCE(iklan_pekerjaan, 0)::bigint AS iklan_pekerjaan, \
                    COALESCE(iklan_pekerja, 0)::bigint AS iklan_pekerja, \
                    COALESCE(iklan_barang_bekas, 0)::bigint AS iklan_barang_bekas, \
                    COALESCE(iklan_pelatihan, 0)::bigint AS iklan_pelatihan, \
                    COALESCE(total_iklan, 0)::bigint AS total_iklan, \
                    COALESCE(baru_30d, 0)::bigint AS baru_30d, \
                    COALESCE(total_users, 0)::bigint AS total_users, \
                    COALESCE(supply_demand_ratio, 0.0)::double precision AS supply_demand_ratio, \
                    COALESCE(canvassing_score, 0)::integer AS canvassing_score \
                FROM analytics.mv_geo_stats \
                WHERE province_id = $1 \
                ORDER BY canvassing_score DESC",
            )
            .bind(pid)
            .fetch_all(&self.pool)
            .await
            .context("gagal query mv_geo_stats")?
        } else {
            sqlx::query_as(
                "SELECT \
                    province_id, \
                    province_name, \
                    COALESCE(iklan_pekerjaan, 0)::bigint AS iklan_pekerjaan, \
                    COALESCE(iklan_pekerja, 0)::bigint AS iklan_pekerja, \
                    COALESCE(iklan_barang_bekas, 0)::bigint AS iklan_barang_bekas, \
                    COALESCE(iklan_pelatihan, 0)::bigint AS iklan_pelatihan, \
                    COALESCE(total_iklan, 0)::bigint AS total_iklan, \
                    COALESCE(baru_30d, 0)::bigint AS baru_30d, \
                    COALESCE(total_users, 0)::bigint AS total_users, \
                    COALESCE(supply_demand_ratio, 0.0)::double precision AS supply_demand_ratio, \
                    COALESCE(canvassing_score, 0)::integer AS canvassing_score \
                FROM analytics.mv_geo_stats \
                ORDER BY canvassing_score DESC",
            )
            .fetch_all(&self.pool)
            .await
            .context("gagal query mv_geo_stats")?
        };
        warn_slow!(start, "mv_geo_stats");
        Ok(rows)
    }

    async fn get_engagement_stats(&self) -> Result<EngagementStats, anyhow::Error> {
        let start = std::time::Instant::now();
        let row: EngagementStats = sqlx::query_as(
            "SELECT \
                COALESCE(messages_7d, 0)::bigint AS messages_7d, \
                COALESCE(messages_30d, 0)::bigint AS messages_30d, \
                COALESCE(conversations_active_7d, 0)::bigint AS conversations_active_7d, \
                COALESCE(notif_sent_7d, 0)::bigint AS notif_sent_7d, \
                COALESCE(notif_read_7d, 0)::bigint AS notif_read_7d, \
                COALESCE(notif_sent_30d, 0)::bigint AS notif_sent_30d, \
                COALESCE(notif_read_30d, 0)::bigint AS notif_read_30d \
            FROM analytics.mv_engagement_stats",
        )
        .fetch_one(&self.pool)
        .await
        .context("gagal query mv_engagement_stats")?;
        warn_slow!(start, "mv_engagement_stats");
        Ok(row)
    }

    async fn refresh_all(&self) -> Result<(), anyhow::Error> {
        let start = std::time::Instant::now();

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_user_stats")
            .execute(&self.pool)
            .await
            .context("gagal refresh mv_user_stats")?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_iklan_stats")
            .execute(&self.pool)
            .await
            .context("gagal refresh mv_iklan_stats")?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_geo_stats")
            .execute(&self.pool)
            .await
            .context("gagal refresh mv_geo_stats")?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY analytics.mv_engagement_stats")
            .execute(&self.pool)
            .await
            .context("gagal refresh mv_engagement_stats")?;

        tracing::info!(
            "analytics materialized views refreshed in {:?}",
            start.elapsed()
        );
        Ok(())
    }
}
