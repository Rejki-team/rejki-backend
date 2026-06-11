use region_service_client::{Region, RegionLevel};

#[derive(Debug, Clone)]
pub struct RegionEntity {
    pub id:        String,
    pub name:      String,
    pub level:     RegionLevel,
    pub parent_id: Option<String>,
}

impl RegionEntity {
    pub fn to_region(self) -> Region {
        Region {
            id:        self.id,
            name:      self.name,
            level:     self.level,
            parent_id: self.parent_id,
        }
    }
}
