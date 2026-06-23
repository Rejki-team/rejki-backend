use crate::domain::entity::{EngagementStats, GeoStats, IklanStats, UserStats};

/// Repository trait untuk query data analytics dari materialized views.
#[allow(async_fn_in_trait)]
pub trait InsightsRepository: Send + Sync {
    /// Ambil statistik pengguna (mv_user_stats).
    async fn get_user_stats(&self) -> Result<UserStats, anyhow::Error>;
    /// Ambil statistik iklan per vertikal (mv_iklan_stats).
    async fn get_iklan_stats(&self) -> Result<Vec<IklanStats>, anyhow::Error>;
    /// Ambil statistik geografis (mv_geo_stats).
    async fn get_geo_stats(
        &self,
        province_id: Option<&str>,
    ) -> Result<Vec<GeoStats>, anyhow::Error>;
    /// Ambil statistik engagement (mv_engagement_stats).
    async fn get_engagement_stats(&self) -> Result<EngagementStats, anyhow::Error>;
    /// Refresh semua materialized views (CONCURRENTLY).
    async fn refresh_all(&self) -> Result<(), anyhow::Error>;
}
