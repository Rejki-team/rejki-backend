use uuid::Uuid;

use super::entity::{Rating, RatingAggregate, RatingArah};

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreateRatingParams<'a> {
    pub iklan_id: Uuid,
    pub penilai_id: Uuid,
    pub dinilai_id: Uuid,
    pub arah: RatingArah,
    pub bintang: i16,
    pub ulasan: Option<&'a str>,
}

#[allow(async_fn_in_trait)]
pub trait RatingRepository: Send + Sync {
    async fn create(&self, params: CreateRatingParams<'_>) -> Result<Rating, anyhow::Error>;

    /// P5.3/P5.5: cek apakah `penilai_id` sudah pernah menilai `dinilai_id` pada
    /// `iklan_id` ini (PRD §5.15: "hanya dapat dikirim satu kali untuk setiap pasangan
    /// pengguna pada satu kode iklan"). Dipanggil SEBELUM `create()` (pola kembar
    /// `has_pending_bider`/`has_conflicting_lamaran`) — unique index di migration
    /// adalah pengaman defense-in-depth terhadap race, bukan jalur utama.
    async fn exists_for_pair(
        &self,
        iklan_id: Uuid,
        penilai_id: Uuid,
        dinilai_id: Uuid,
    ) -> Result<bool, anyhow::Error>;

    /// P5.4: agregasi rating "keaktifan" milik satu user (lihat doc `RatingAggregate`).
    async fn get_aggregate_for_user(&self, user_id: Uuid)
        -> Result<RatingAggregate, anyhow::Error>;
}
