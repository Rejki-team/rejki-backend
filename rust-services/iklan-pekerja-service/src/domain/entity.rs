use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IklanPekerja {
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
    pub updated_at: DateTime<Utc>,
}
