use std::sync::Arc;
use uuid::Uuid;

use super::dto::{CreateRatingInput, RatingAggregateResponse, RatingResponse};
use crate::domain::entity::{Rating, RatingArah};
use crate::domain::repository::{CreateRatingParams, RatingRepository};
use iklan_pekerjaan_service_client::IklanPekerjaanClient;

pub struct RatingService<R: RatingRepository> {
    repo: Arc<R>,
    iklan_pekerjaan_client: Option<Arc<dyn IklanPekerjaanClient>>,
}

impl<R: RatingRepository> RatingService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            iklan_pekerjaan_client: None,
        }
    }

    pub fn with_iklan_pekerjaan_client(mut self, c: Arc<dyn IklanPekerjaanClient>) -> Self {
        self.iklan_pekerjaan_client = Some(c);
        self
    }

    /// PRD §5.15: rating dua arah, hanya terbuka setelah aktifitas Selesai, satu kali
    /// per pasangan per kode iklan, tidak dapat diubah.
    pub async fn create_rating(
        &self,
        penilai_id: Uuid,
        input: CreateRatingInput,
    ) -> Result<RatingResponse, anyhow::Error> {
        let arah = RatingArah::parse(&input.arah)
            .ok_or_else(|| anyhow::anyhow!("arah tidak dikenal: {}", input.arah))?;

        if input.dinilai_id == penilai_id {
            return Err(anyhow::anyhow!(
                "tidak dapat memberi rating untuk diri sendiri"
            ));
        }

        // (poster_id, pelamar_id) ditentukan dari arah — lihat doc `IklanPekerjaanClient::is_lamaran_selesai`.
        let (poster_id, pelamar_id) = match arah {
            RatingArah::PelamarKePemberiKerja => (input.dinilai_id, penilai_id),
            RatingArah::PemberiKerjaKePelamar => (penilai_id, input.dinilai_id),
        };

        let client = self
            .iklan_pekerjaan_client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("layanan validasi lamaran tidak tersedia"))?;
        let selesai = client
            .is_lamaran_selesai(input.iklan_id, poster_id, pelamar_id)
            .await
            .map_err(|_| anyhow::anyhow!("gagal memvalidasi status lamaran"))?;
        if !selesai {
            return Err(anyhow::anyhow!(
                "lamaran belum berstatus Selesai, rating belum dapat diberikan"
            ));
        }

        if self
            .repo
            .exists_for_pair(input.iklan_id, penilai_id, input.dinilai_id)
            .await?
        {
            return Err(anyhow::anyhow!(
                "Anda sudah memberikan rating untuk pasangan ini pada iklan ini"
            ));
        }

        let ulasan = input.ulasan.as_deref().map(ammonia::clean_text);

        let rating = self
            .repo
            .create(CreateRatingParams {
                iklan_id: input.iklan_id,
                penilai_id,
                dinilai_id: input.dinilai_id,
                arah,
                bintang: input.bintang,
                ulasan: ulasan.as_deref(),
            })
            .await?;

        Ok(to_response(rating))
    }

    pub async fn get_aggregate(
        &self,
        user_id: Uuid,
    ) -> Result<RatingAggregateResponse, anyhow::Error> {
        let agg = self.repo.get_aggregate_for_user(user_id).await?;
        Ok(RatingAggregateResponse {
            user_id,
            average: agg.average,
            count: agg.count,
        })
    }
}

fn to_response(r: Rating) -> RatingResponse {
    RatingResponse {
        id: r.id,
        iklan_id: r.iklan_id,
        penilai_id: r.penilai_id,
        dinilai_id: r.dinilai_id,
        arah: r.arah.as_str().to_string(),
        bintang: r.bintang,
        ulasan: r.ulasan,
        created_at: r.created_at,
    }
}
