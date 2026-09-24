use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Arah penilaian (F-17, Kelompok 3 Phase 5, PRD §5.15) — rating dua arah pada modul
/// Iklan Pekerjaan: pelamar menilai pemberi kerja, dan sebaliknya.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingArah {
    PelamarKePemberiKerja,
    PemberiKerjaKePelamar,
}

impl RatingArah {
    pub fn as_str(&self) -> &'static str {
        match self {
            RatingArah::PelamarKePemberiKerja => "pelamar_ke_pemberi_kerja",
            RatingArah::PemberiKerjaKePelamar => "pemberi_kerja_ke_pelamar",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pelamar_ke_pemberi_kerja" => Some(RatingArah::PelamarKePemberiKerja),
            "pemberi_kerja_ke_pelamar" => Some(RatingArah::PemberiKerjaKePelamar),
            _ => None,
        }
    }
}

impl std::fmt::Display for RatingArah {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Satu baris rating (PRD §5.15: bintang 1-5, ulasan opsional 20-255 karakter,
/// hanya sekali per pasangan `(iklan_id, penilai_id, dinilai_id)`, tidak dapat diubah
/// — tidak ada method `update` di repository, sengaja).
#[derive(Debug, Clone)]
pub struct Rating {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub penilai_id: Uuid,
    pub dinilai_id: Uuid,
    pub arah: RatingArah,
    pub bintang: i16,
    pub ulasan: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Agregasi rating "keaktifan" untuk satu user (PRD §5.15: "gabungan rating yang
/// diterima pengguna dari SELURUH modul yang memiliki fitur rating"). Query agregasi
/// (`WHERE dinilai_id=$1`, tanpa filter modul/sumber) sengaja tidak membedakan modul —
/// baris rating dari modul manapun (saat ini hanya Iklan Pekerjaan) otomatis terhitung,
/// tidak perlu migrasi skema saat modul lain menambah rating nanti (P5.4).
#[derive(Debug, Clone, Copy)]
pub struct RatingAggregate {
    pub average: f64,
    pub count: i64,
}
