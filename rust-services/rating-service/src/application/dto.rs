use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Input "Kirim Rating" (POST /rating, PRD §5.15). `penilai_id` diambil dari JWT
/// (`AuthClaims`), TIDAK dari body — mencegah user menyamar sebagai orang lain.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateRatingInput {
    pub iklan_id: Uuid,
    pub dinilai_id: Uuid,
    /// "pelamar_ke_pemberi_kerja" | "pemberi_kerja_ke_pelamar"
    #[validate(custom(function = "validate_arah"))]
    pub arah: String,
    #[validate(range(min = 1, max = 5))]
    pub bintang: i16,
    /// Opsional; PRD §5.15: 20-255 karakter BILA diisi.
    #[validate(length(min = 20, max = 255))]
    pub ulasan: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RatingResponse {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub penilai_id: Uuid,
    pub dinilai_id: Uuid,
    pub arah: String,
    pub bintang: i16,
    pub ulasan: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// GET /rating/profil/{user_id} (P5.4) — "Rating keaktifan" gabungan.
#[derive(Debug, Serialize)]
pub struct RatingAggregateResponse {
    pub user_id: Uuid,
    pub average: f64,
    pub count: i64,
}

fn validate_arah(s: &str) -> Result<(), validator::ValidationError> {
    if s == "pelamar_ke_pemberi_kerja" || s == "pemberi_kerja_ke_pelamar" {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("invalid_arah");
        err.message = Some(format!("arah tidak dikenal: {s}").into());
        Err(err)
    }
}
