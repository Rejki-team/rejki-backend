use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct IklanPekerjaanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPekerjaanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 2, max = 200))]
    pub perusahaan: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub gaji_min: Option<i64>,
    pub gaji_max: Option<i64>,
    pub tipe: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
