use std::sync::Arc;

use region_service_client::{Region, RegionClient, RegionClientError};
use crate::application::service::RegionService;
use crate::infrastructure::PgRegionRepository;

/// Implementasi RegionClient untuk mode in-process (Modular Monolith).
#[derive(Clone)]
pub struct RegionInProcessClient {
    svc: Arc<RegionService<PgRegionRepository>>,
}

impl RegionInProcessClient {
    pub fn new(svc: Arc<RegionService<PgRegionRepository>>) -> Self {
        Self { svc }
    }
}

#[async_trait::async_trait]
impl RegionClient for RegionInProcessClient {
    async fn list_provinces(&self) -> Result<Vec<Region>, RegionClientError> {
        self.svc.list_provinces().await.map_err(|_| RegionClientError::Unavailable)
    }

    async fn list_regencies(&self, province_id: &str) -> Result<Vec<Region>, RegionClientError> {
        self.svc.list_regencies(province_id).await.map_err(|_| RegionClientError::Unavailable)
    }

    async fn list_districts(&self, regency_id: &str) -> Result<Vec<Region>, RegionClientError> {
        self.svc.list_districts(regency_id).await.map_err(|_| RegionClientError::Unavailable)
    }

    async fn list_villages(&self, district_id: &str) -> Result<Vec<Region>, RegionClientError> {
        self.svc.list_villages(district_id).await.map_err(|_| RegionClientError::Unavailable)
    }

    async fn get_region(&self, id: &str) -> Result<Region, RegionClientError> {
        self.svc
            .get_region(id)
            .await
            .map_err(|_| RegionClientError::Unavailable)?
            .ok_or(RegionClientError::NotFound)
    }

    async fn validate_chain(
        &self,
        province_id: &str,
        regency_id:  &str,
        district_id: &str,
        village_id:  &str,
    ) -> Result<bool, RegionClientError> {
        self.svc
            .validate_chain(province_id, regency_id, district_id, village_id)
            .await
            .map_err(|_| RegionClientError::Unavailable)
    }
}
