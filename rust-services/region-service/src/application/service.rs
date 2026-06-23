use std::sync::Arc;

use crate::domain::repository::RegionRepository;
use region_service_client::{Region, RegionLevel};

pub struct RegionService<R: RegionRepository> {
    repo: Arc<R>,
}

impl<R: RegionRepository> RegionService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn list_provinces(&self) -> Result<Vec<Region>, anyhow::Error> {
        Ok(self
            .repo
            .list_by_level(RegionLevel::Province, None)
            .await?
            .into_iter()
            .map(|e| e.to_region())
            .collect())
    }

    pub async fn list_regencies(&self, province_id: &str) -> Result<Vec<Region>, anyhow::Error> {
        Ok(self
            .repo
            .list_by_level(RegionLevel::Regency, Some(province_id))
            .await?
            .into_iter()
            .map(|e| e.to_region())
            .collect())
    }

    pub async fn list_districts(&self, regency_id: &str) -> Result<Vec<Region>, anyhow::Error> {
        Ok(self
            .repo
            .list_by_level(RegionLevel::District, Some(regency_id))
            .await?
            .into_iter()
            .map(|e| e.to_region())
            .collect())
    }

    pub async fn list_villages(&self, district_id: &str) -> Result<Vec<Region>, anyhow::Error> {
        Ok(self
            .repo
            .list_by_level(RegionLevel::Village, Some(district_id))
            .await?
            .into_iter()
            .map(|e| e.to_region())
            .collect())
    }

    pub async fn get_region(&self, id: &str) -> Result<Option<Region>, anyhow::Error> {
        Ok(self.repo.get_by_id(id).await?.map(|e| e.to_region()))
    }

    pub async fn validate_chain(
        &self,
        province_id: &str,
        regency_id: &str,
        district_id: &str,
        village_id: &str,
    ) -> Result<bool, anyhow::Error> {
        self.repo
            .validate_chain(province_id, regency_id, district_id, village_id)
            .await
    }
}
