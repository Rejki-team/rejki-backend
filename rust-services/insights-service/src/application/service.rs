use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use crate::application::dto::{
    CanvassingResponse, CanvassingSummaryResponse, EngagementStatsResponse, GeoStatsResponse,
    IklanStatsResponse, RefreshResponse, UserStatsResponse,
};
use crate::domain::repository::InsightsRepository;

/// TTL cache dalam detik.
const CACHE_TTL_SECS: u64 = 300;

/// Key untuk cache.
type CacheKey = String;

/// Value cache dengan timestamp.
struct CacheValue {
    data: serde_json::Value,
    expires_at: Instant,
}

/// InsightsService — orchestrator untuk analytics dengan cache.
pub struct InsightsService<R: InsightsRepository> {
    repo: Arc<R>,
    cache: Arc<RwLock<HashMap<CacheKey, CacheValue>>>,
}

impl<R: InsightsRepository> InsightsService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_cached(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    async fn set_cache(&self, key: String, value: serde_json::Value) {
        let mut cache = self.cache.write().await;
        cache.insert(
            key,
            CacheValue {
                data: value,
                expires_at: Instant::now() + std::time::Duration::from_secs(CACHE_TTL_SECS),
            },
        );
    }

    pub async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::info!("insights cache cleared");
    }

    // ── User Stats ──────────────────────────────────────────────

    pub async fn get_user_stats(&self) -> Result<UserStatsResponse, anyhow::Error> {
        let cache_key = "user_stats".to_string();
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(serde_json::from_value(cached)?);
        }
        let stats = self.repo.get_user_stats().await?;
        let resp = UserStatsResponse::from(stats);
        self.set_cache(cache_key, serde_json::to_value(&resp)?)
            .await;
        Ok(resp)
    }

    // ── Iklan Stats ─────────────────────────────────────────────

    pub async fn get_iklan_stats(
        &self,
        vertikal: Option<&str>,
    ) -> Result<Vec<IklanStatsResponse>, anyhow::Error> {
        let cache_key = format!("iklan_stats:{}", vertikal.unwrap_or("all"));
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(serde_json::from_value(cached)?);
        }
        let stats = self.repo.get_iklan_stats().await?;
        let mut resps: Vec<IklanStatsResponse> = stats.into_iter().map(|s| s.into()).collect();
        if let Some(v) = vertikal {
            resps.retain(|r| r.vertikal == v);
        }
        self.set_cache(cache_key, serde_json::to_value(&resps)?)
            .await;
        Ok(resps)
    }

    // ── Geo Stats ───────────────────────────────────────────────

    pub async fn get_geo_stats(
        &self,
        province_id: Option<&str>,
    ) -> Result<Vec<GeoStatsResponse>, anyhow::Error> {
        let cache_key = format!("geo_stats:{}", province_id.unwrap_or("all"));
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(serde_json::from_value(cached)?);
        }
        let stats = self.repo.get_geo_stats(province_id).await?;
        let resps: Vec<GeoStatsResponse> = stats.into_iter().map(|s| s.into()).collect();
        self.set_cache(cache_key, serde_json::to_value(&resps)?)
            .await;
        Ok(resps)
    }

    // ── Engagement Stats ────────────────────────────────────────

    pub async fn get_engagement_stats(&self) -> Result<EngagementStatsResponse, anyhow::Error> {
        let cache_key = "engagement_stats".to_string();
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(serde_json::from_value(cached)?);
        }
        let stats = self.repo.get_engagement_stats().await?;
        let resp = EngagementStatsResponse::from(stats);
        self.set_cache(cache_key, serde_json::to_value(&resp)?)
            .await;
        Ok(resp)
    }

    // ── Canvassing ──────────────────────────────────────────────

    pub async fn get_canvassing(&self) -> Result<CanvassingResponse, anyhow::Error> {
        let cache_key = "canvassing".to_string();
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(serde_json::from_value(cached)?);
        }
        let stats = self.repo.get_geo_stats(None).await?;
        let provinces: Vec<GeoStatsResponse> = stats.into_iter().map(|s| s.into()).collect();

        let total = provinces.len();
        let high = provinces
            .iter()
            .filter(|p| p.canvassing_score >= 60)
            .count();
        let medium = provinces
            .iter()
            .filter(|p| p.canvassing_score >= 40 && p.canvassing_score < 60)
            .count();
        let avg = if total > 0 {
            let sum: i32 = provinces.iter().map(|p| p.canvassing_score).sum();
            (sum as f64 / total as f64 * 10.0).round() / 10.0
        } else {
            0.0
        };

        let resp = CanvassingResponse {
            provinces,
            summary: CanvassingSummaryResponse {
                total_provinces_analyzed: total,
                high_priority_count: high,
                medium_priority_count: medium,
                avg_canvassing_score: avg,
            },
        };
        self.set_cache(cache_key, serde_json::to_value(&resp)?)
            .await;
        Ok(resp)
    }

    // ── Refresh ─────────────────────────────────────────────────

    pub async fn refresh(&self) -> Result<RefreshResponse, anyhow::Error> {
        self.repo.refresh_all().await?;
        self.invalidate_cache().await;
        Ok(RefreshResponse {
            status: "ok".to_string(),
            message: "Data analytics sedang diperbarui. Cache telah dibersihkan.".to_string(),
        })
    }
}
