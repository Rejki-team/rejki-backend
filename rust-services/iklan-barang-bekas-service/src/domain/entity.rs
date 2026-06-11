use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IklanBarangBekas {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub judul: String,
    pub deskripsi: String,
    pub harga: i64,
    pub kondisi: String, // "baru" | "sangat_baik" | "baik" | "cukup"
    pub lokasi: Option<String>,
    pub foto_urls: Vec<String>,
    pub is_sold: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
