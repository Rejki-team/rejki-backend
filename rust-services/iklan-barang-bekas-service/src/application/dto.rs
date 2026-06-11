use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct IklanBarangBekasResponse {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub judul: String,
    pub deskripsi: String,
    pub harga: i64,
    pub kondisi: String,
    pub lokasi: Option<String>,
    pub is_sold: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanBarangBekasInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    pub deskripsi: String,
    #[validate(range(min = 0))]
    pub harga: i64,
    pub kondisi: String,
    pub lokasi: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
