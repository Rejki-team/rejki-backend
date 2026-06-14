/// Tingkat wilayah administratif.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionLevel {
    Province,
    Regency,
    District,
    Village,
}

impl RegionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RegionLevel::Province => "province",
            RegionLevel::Regency => "regency",
            RegionLevel::District => "district",
            RegionLevel::Village => "village",
        }
    }
}

/// Satu entri wilayah. `id` = kode wilayah resmi; `name` = nama; `parent_id` = induk (None untuk provinsi).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub level: RegionLevel,
    pub parent_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegionClientError {
    #[error("region not found")]
    NotFound,
    #[error("region service unavailable")]
    Unavailable,
}

/// Kontrak publik untuk data wilayah — dipanggil in-process oleh domain lain.
/// Implementasinya ada di region-service.
#[async_trait::async_trait]
pub trait RegionClient: Send + Sync {
    /// Daftar provinsi (seluruh Indonesia — data kecil).
    async fn list_provinces(&self) -> Result<Vec<Region>, RegionClientError>;

    /// Daftar kabupaten/kota berdasarkan provinsi.
    async fn list_regencies(&self, province_id: &str) -> Result<Vec<Region>, RegionClientError>;

    /// Daftar kecamatan berdasarkan kab/kota.
    async fn list_districts(&self, regency_id: &str) -> Result<Vec<Region>, RegionClientError>;

    /// Daftar kelurahan/desa berdasarkan kecamatan.
    async fn list_villages(&self, district_id: &str) -> Result<Vec<Region>, RegionClientError>;

    /// Ambil satu wilayah berdasarkan kode.
    async fn get_region(&self, id: &str) -> Result<Region, RegionClientError>;

    /// Validasi bahwa rantai provinsi→kab→kec→kel konsisten secara berjenjang.
    async fn validate_chain(
        &self,
        province_id: &str,
        regency_id: &str,
        district_id: &str,
        village_id: &str,
    ) -> Result<bool, RegionClientError>;
}
