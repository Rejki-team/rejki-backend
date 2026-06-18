#[cfg(test)]
mod tests {
    use crate::application::service::RegionService;
    use crate::domain::entity::RegionEntity;
    use crate::domain::repository::RegionRepository;
    use region_service_client::{Region, RegionLevel};
    use std::sync::Mutex;

    // ── MockRegionRepository ────────────────────────────────────────────────────

    struct MockRegionRepository {
        regions: Mutex<Vec<RegionEntity>>,
    }

    impl MockRegionRepository {
        fn new() -> Self {
            Self {
                regions: Mutex::new(vec![]),
            }
        }

        fn seed_with(&self, regions: Vec<RegionEntity>) {
            *self.regions.lock().unwrap() = regions;
        }
    }

    #[allow(async_fn_in_trait)]
    impl RegionRepository for MockRegionRepository {
        async fn list_by_level(
            &self,
            level: RegionLevel,
            parent_id: Option<&str>,
        ) -> Result<Vec<RegionEntity>, anyhow::Error> {
            Ok(self
                .regions
                .lock()
                .unwrap()
                .iter()
                .filter(|r| r.level == level)
                .filter(|r| match parent_id {
                    Some(pid) => r.parent_id.as_deref() == Some(pid),
                    None => true,
                })
                .cloned()
                .collect())
        }

        async fn get_by_id(&self, id: &str) -> Result<Option<RegionEntity>, anyhow::Error> {
            Ok(self
                .regions
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.id == id)
                .cloned())
        }

        async fn validate_chain(
            &self,
            province_id: &str,
            regency_id: &str,
            district_id: &str,
            village_id: &str,
        ) -> Result<bool, anyhow::Error> {
            let regs = self.regions.lock().unwrap();
            let regency = regs
                .iter()
                .find(|r| r.id == regency_id && r.parent_id.as_deref() == Some(province_id));
            if regency.is_none() {
                return Ok(false);
            }
            let district = regs
                .iter()
                .find(|r| r.id == district_id && r.parent_id.as_deref() == Some(regency_id));
            if district.is_none() {
                return Ok(false);
            }
            let village = regs
                .iter()
                .find(|r| r.id == village_id && r.parent_id.as_deref() == Some(district_id));
            Ok(village.is_some())
        }
    }

    fn svc() -> RegionService<MockRegionRepository> {
        let repo = std::sync::Arc::new(MockRegionRepository::new());
        RegionService::new(repo)
    }

    fn seeded_svc() -> (
        RegionService<MockRegionRepository>,
        std::sync::Arc<MockRegionRepository>,
    ) {
        let repo = std::sync::Arc::new(MockRegionRepository::new());
        repo.seed_with(vec![
            RegionEntity {
                id: "11".into(),
                name: "Jawa Barat".into(),
                level: RegionLevel::Province,
                parent_id: None,
            },
            RegionEntity {
                id: "1101".into(),
                name: "Kab. Bogor".into(),
                level: RegionLevel::Regency,
                parent_id: Some("11".into()),
            },
            RegionEntity {
                id: "110101".into(),
                name: "Cibinong".into(),
                level: RegionLevel::District,
                parent_id: Some("1101".into()),
            },
            RegionEntity {
                id: "1101012001".into(),
                name: "Kelurahan X".into(),
                level: RegionLevel::Village,
                parent_id: Some("110101".into()),
            },
        ]);
        (RegionService::new(repo.clone()), repo)
    }

    // ── list_provinces ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_provinces_given_seeded_data_when_list_then_returns_provinces() {
        let (s, _repo) = seeded_svc();
        let provinces = s.list_provinces().await.unwrap();
        assert_eq!(provinces.len(), 1);
        assert_eq!(provinces[0].name, "Jawa Barat");
        assert_eq!(provinces[0].level, RegionLevel::Province);
    }

    // ── list_regencies ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_regencies_given_valid_province_when_list_then_returns_regencies() {
        let (s, _repo) = seeded_svc();
        let regencies = s.list_regencies("11").await.unwrap();
        assert_eq!(regencies.len(), 1);
        assert_eq!(regencies[0].name, "Kab. Bogor");
        assert_eq!(regencies[0].level, RegionLevel::Regency);
    }

    #[tokio::test]
    async fn test_list_regencies_given_invalid_province_when_list_then_returns_empty() {
        let (s, _repo) = seeded_svc();
        let regencies = s.list_regencies("99").await.unwrap();
        assert!(regencies.is_empty());
    }

    // ── list_districts ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_districts_given_valid_regency_when_list_then_returns_districts() {
        let (s, _repo) = seeded_svc();
        let districts = s.list_districts("1101").await.unwrap();
        assert_eq!(districts.len(), 1);
        assert_eq!(districts[0].name, "Cibinong");
    }

    // ── list_villages ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_villages_given_valid_district_when_list_then_returns_villages() {
        let (s, _repo) = seeded_svc();
        let villages = s.list_villages("110101").await.unwrap();
        assert_eq!(villages.len(), 1);
        assert_eq!(villages[0].name, "Kelurahan X");
    }

    // ── get_region ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_region_given_valid_id_when_get_then_returns_region() {
        let (s, _repo) = seeded_svc();
        let region = s.get_region("1101").await.unwrap();
        assert!(region.is_some());
        assert_eq!(region.unwrap().name, "Kab. Bogor");
    }

    #[tokio::test]
    async fn test_get_region_given_invalid_id_when_get_then_returns_none() {
        let (s, _repo) = seeded_svc();
        assert!(s.get_region("9999").await.unwrap().is_none());
    }

    // ── validate_chain ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_validate_chain_given_valid_chain_when_validate_then_returns_true() {
        let (s, _repo) = seeded_svc();
        assert!(s
            .validate_chain("11", "1101", "110101", "1101012001")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_validate_chain_given_broken_chain_when_validate_then_returns_false() {
        let (s, _repo) = seeded_svc();
        // regency_id "1101" parent is "11" — claiming parent is "99" breaks chain
        assert!(!s
            .validate_chain("99", "1101", "110101", "1101012001")
            .await
            .unwrap());
    }

    // ── RegionLevel ─────────────────────────────────────────────────────────────

    #[test]
    fn test_region_level_as_str() {
        assert_eq!(RegionLevel::Province.as_str(), "province");
        assert_eq!(RegionLevel::Regency.as_str(), "regency");
        assert_eq!(RegionLevel::District.as_str(), "district");
        assert_eq!(RegionLevel::Village.as_str(), "village");
    }

    // ── RegionEntity mapping ────────────────────────────────────────────────────

    #[test]
    fn test_region_entity_to_region() {
        let entity = RegionEntity {
            id: "11".into(),
            name: "Jawa Barat".into(),
            level: RegionLevel::Province,
            parent_id: None,
        };
        let region: Region = entity.to_region();
        assert_eq!(region.id, "11");
        assert_eq!(region.name, "Jawa Barat");
        assert_eq!(region.level, RegionLevel::Province);
        assert!(region.parent_id.is_none());
    }
}
