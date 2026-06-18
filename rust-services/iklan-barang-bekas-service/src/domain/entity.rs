use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Status moderasi iklan (state machine admin).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus {
    #[default]
    Active,
    SuspendedTemp,
    SuspendedPermanent,
}

impl ModerationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationStatus::Active => "active",
            ModerationStatus::SuspendedTemp => "suspended_temp",
            ModerationStatus::SuspendedPermanent => "suspended_permanent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(ModerationStatus::Active),
            "suspended_temp" => Some(ModerationStatus::SuspendedTemp),
            "suspended_permanent" => Some(ModerationStatus::SuspendedPermanent),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModerationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Model gratis/donasi (extend-barang-bekas-gratis-model D1-D2).
/// Tidak ada harga/kondisi — fokus pada jenis barang, jumlah, lokasi pengambilan,
/// dan status ketersediaan terpisah dari status moderasi.
#[derive(Debug, Clone)]
pub struct IklanBarangBekas {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub judul: String,
    pub deskripsi: String,
    /// Jenis barang: "bekas" | "baru"
    pub jenis_barang: String,
    /// Jumlah barang (≥1)
    pub jumlah: i32,
    /// Lokasi pengambilan barang (wajib diisi)
    pub lokasi_pengambilan: String,
    pub lokasi: Option<String>,
    pub foto_urls: Vec<String>,
    /// Status ketersediaan: "tersedia" | "sudah_diambil"
    pub availability_status: AvailabilityStatus,
    pub moderation_status: ModerationStatus,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status ketersediaan barang donasi — terpisah dari status moderasi (D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvailabilityStatus {
    #[default]
    Tersedia,
    SudahDiambil,
}

impl AvailabilityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AvailabilityStatus::Tersedia => "tersedia",
            AvailabilityStatus::SudahDiambil => "sudah_diambil",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "tersedia" => Some(AvailabilityStatus::Tersedia),
            "sudah_diambil" => Some(AvailabilityStatus::SudahDiambil),
            _ => None,
        }
    }
}

impl std::fmt::Display for AvailabilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for AvailabilityStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for AvailabilityStatus {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        AvailabilityStatus::parse(&s).ok_or_else(|| {
            serde::de::Error::custom(format!("availability_status tidak dikenal: {s}"))
        })
    }
}

/// Riwayat suspend iklan (audit trail).
#[derive(Debug, Clone)]
pub struct IklanSuspension {
    pub id: Uuid,
    pub iklan_id: Uuid,
    pub is_permanent: bool,
    pub reason: String,
    pub evidence_object_key: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}
