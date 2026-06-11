use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct IklanPelatihanResponse {
    pub id: Uuid,
    pub poster_id: Uuid,
    pub judul: String,
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIklanPelatihanInput {
    #[validate(length(min = 3, max = 200))]
    pub judul: String,
    #[validate(length(min = 1))]
    pub penyelenggara: String,
    pub deskripsi: String,
    pub lokasi: Option<String>,
    pub harga: Option<i64>,
    pub tanggal_mulai: Option<DateTime<Utc>>,
    pub tanggal_selesai: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
