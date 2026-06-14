use super::entity::RegionEntity;

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Repository dipakai in-process; cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait RegionRepository: Send + Sync {
    async fn list_by_level(
        &self,
        level: region_service_client::RegionLevel,
        parent_id: Option<&str>,
    ) -> Result<Vec<RegionEntity>, anyhow::Error>;

    async fn get_by_id(&self, id: &str) -> Result<Option<RegionEntity>, anyhow::Error>;

    /// Validasi rantai wilayah berjenjang.
    async fn validate_chain(
        &self,
        province_id: &str,
        regency_id: &str,
        district_id: &str,
        village_id: &str,
    ) -> Result<bool, anyhow::Error>;
}
