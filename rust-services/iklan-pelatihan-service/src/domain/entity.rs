use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IklanPelatihan {
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
    pub updated_at: DateTime<Utc>,
}
