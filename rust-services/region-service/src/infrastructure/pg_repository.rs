use sqlx::PgPool;

use crate::domain::entity::RegionEntity;
use crate::domain::repository::RegionRepository;
use region_service_client::RegionLevel;

pub struct PgRegionRepository {
    pool: PgPool,
}

impl PgRegionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Map row (id, name) + level + optional parent ke RegionEntity.
fn region_from_row(
    id: String,
    name: String,
    level: RegionLevel,
    parent_id: Option<&str>,
) -> RegionEntity {
    RegionEntity {
        id,
        name,
        level,
        parent_id: parent_id.map(|s| s.to_owned()),
    }
}

impl RegionRepository for PgRegionRepository {
    async fn list_by_level(
        &self,
        level: RegionLevel,
        parent_id: Option<&str>,
    ) -> Result<Vec<RegionEntity>, anyhow::Error> {
        let rows: Vec<RegionEntity> = match level {
            RegionLevel::Province => {
                sqlx::query!("SELECT id, name FROM region.province ORDER BY name")
                    .fetch_all(&self.pool)
                    .await?
                    .into_iter()
                    .map(|r| region_from_row(r.id, r.name, RegionLevel::Province, None))
                    .collect()
            }
            RegionLevel::Regency => {
                let pid = parent_id.ok_or_else(|| anyhow::anyhow!("province_id wajib"))?;
                sqlx::query!(
                    "SELECT id, name FROM region.regency WHERE province_id = $1 ORDER BY name",
                    pid
                )
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|r| region_from_row(r.id, r.name, RegionLevel::Regency, Some(pid)))
                .collect()
            }
            RegionLevel::District => {
                let pid = parent_id.ok_or_else(|| anyhow::anyhow!("regency_id wajib"))?;
                sqlx::query!(
                    "SELECT id, name FROM region.district WHERE regency_id = $1 ORDER BY name",
                    pid
                )
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|r| region_from_row(r.id, r.name, RegionLevel::District, Some(pid)))
                .collect()
            }
            RegionLevel::Village => {
                let pid = parent_id.ok_or_else(|| anyhow::anyhow!("district_id wajib"))?;
                sqlx::query!(
                    "SELECT id, name FROM region.village WHERE district_id = $1 ORDER BY name",
                    pid
                )
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .map(|r| region_from_row(r.id, r.name, RegionLevel::Village, Some(pid)))
                .collect()
            }
        };
        Ok(rows)
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<RegionEntity>, anyhow::Error> {
        // Single query UNION ALL — 4 sequential queries jadi 1 round trip
        let row: Option<(String, String, String)> = sqlx::query_as(
            r#"SELECT id, name, 'village' FROM region.village WHERE id = $1
               UNION ALL
               SELECT id, name, 'district' FROM region.district WHERE id = $1
               UNION ALL
               SELECT id, name, 'regency' FROM region.regency WHERE id = $1
               UNION ALL
               SELECT id, name, 'province' FROM region.province WHERE id = $1
               LIMIT 1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, name, level)| {
            let level = match level.as_str() {
                "village" => RegionLevel::Village,
                "district" => RegionLevel::District,
                "regency" => RegionLevel::Regency,
                _ => RegionLevel::Province,
            };
            region_from_row(id, name, level, None)
        }))
    }

    async fn validate_chain(
        &self,
        province_id: &str,
        regency_id: &str,
        district_id: &str,
        village_id: &str,
    ) -> Result<bool, anyhow::Error> {
        Ok(sqlx::query_scalar!(
            "SELECT EXISTS(
                    SELECT 1 FROM region.village v
                    JOIN region.district d ON d.id = v.district_id
                    JOIN region.regency  r ON r.id = d.regency_id
                    WHERE v.id = $1 AND d.id = $2 AND r.id = $3 AND r.province_id = $4
                ) AS \"exists!\"",
            village_id,
            district_id,
            regency_id,
            province_id
        )
        .fetch_one(&self.pool)
        .await?)
    }
}
