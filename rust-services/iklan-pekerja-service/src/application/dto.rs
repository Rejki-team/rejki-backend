use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct IklanPekerjaResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPekerjaInput {
    #[validate(length(min = 2, max = 200))]
    pub nama: String,
    pub keahlian: Vec<String>,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub tarif_min: Option<i64>,
    pub tarif_max: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
