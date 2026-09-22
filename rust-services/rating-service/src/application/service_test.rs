#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::CreateRatingInput;
    use crate::application::service::RatingService;
    use crate::domain::entity::{Rating, RatingAggregate};
    use crate::domain::repository::{CreateRatingParams, RatingRepository};
    use iklan_pekerjaan_service_client::{
        IklanPekerjaanClient, IklanPekerjaanClientError, IklanPekerjaanSummary,
    };

    // ── MockRatingRepository ─────────────────────────────────────────────────────

    struct MockRatingRepository {
        rows: Mutex<Vec<Rating>>,
    }

    impl MockRatingRepository {
        fn new() -> Self {
            Self {
                rows: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl RatingRepository for MockRatingRepository {
        async fn create(&self, params: CreateRatingParams<'_>) -> Result<Rating, anyhow::Error> {
            let rating = Rating {
                id: Uuid::now_v7(),
                iklan_id: params.iklan_id,
                penilai_id: params.penilai_id,
                dinilai_id: params.dinilai_id,
                arah: params.arah,
                bintang: params.bintang,
                ulasan: params.ulasan.map(String::from),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.rows.lock().unwrap().push(rating.clone());
            Ok(rating)
        }

        async fn exists_for_pair(
            &self,
            iklan_id: Uuid,
            penilai_id: Uuid,
            dinilai_id: Uuid,
        ) -> Result<bool, anyhow::Error> {
            Ok(self.rows.lock().unwrap().iter().any(|r| {
                r.iklan_id == iklan_id && r.penilai_id == penilai_id && r.dinilai_id == dinilai_id
            }))
        }

        async fn get_aggregate_for_user(
            &self,
            user_id: Uuid,
        ) -> Result<RatingAggregate, anyhow::Error> {
            let rows = self.rows.lock().unwrap();
            let mine: Vec<&Rating> = rows.iter().filter(|r| r.dinilai_id == user_id).collect();
            if mine.is_empty() {
                return Ok(RatingAggregate {
                    average: 0.0,
                    count: 0,
                });
            }
            let sum: i64 = mine.iter().map(|r| r.bintang as i64).sum();
            Ok(RatingAggregate {
                average: sum as f64 / mine.len() as f64,
                count: mine.len() as i64,
            })
        }
    }

    // ── MockIklanPekerjaanClient ────────────────────────────────────────────────

    struct MockIklanPekerjaanClient {
        selesai: bool,
    }

    #[async_trait::async_trait]
    impl IklanPekerjaanClient for MockIklanPekerjaanClient {
        async fn get_summary(
            &self,
            _id: Uuid,
        ) -> Result<IklanPekerjaanSummary, IklanPekerjaanClientError> {
            Err(IklanPekerjaanClientError::NotFound)
        }
        async fn exists(&self, _id: Uuid) -> Result<bool, IklanPekerjaanClientError> {
            Ok(true)
        }
        async fn is_lamaran_selesai(
            &self,
            _iklan_id: Uuid,
            _poster_id: Uuid,
            _pelamar_id: Uuid,
        ) -> Result<bool, IklanPekerjaanClientError> {
            Ok(self.selesai)
        }
        async fn suspend(
            &self,
            _iklan_id: Uuid,
            _is_permanent: bool,
            _reason: &str,
            _expires_at: Option<chrono::DateTime<chrono::Utc>>,
            _admin_id: Uuid,
        ) -> Result<(), IklanPekerjaanClientError> {
            unimplemented!("tidak dipakai test rating-service")
        }
    }

    fn svc(selesai: bool) -> RatingService<MockRatingRepository> {
        RatingService::new(std::sync::Arc::new(MockRatingRepository::new()))
            .with_iklan_pekerjaan_client(std::sync::Arc::new(MockIklanPekerjaanClient { selesai }))
    }

    fn input(dinilai_id: Uuid, arah: &str) -> CreateRatingInput {
        CreateRatingInput {
            iklan_id: Uuid::now_v7(),
            dinilai_id,
            arah: arah.to_string(),
            bintang: 5,
            ulasan: Some("Pekerja yang sangat baik dan tepat waktu".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_rating_given_lamaran_selesai_when_submitted_then_succeeds() {
        let s = svc(true);
        let penilai = Uuid::now_v7();
        let dinilai = Uuid::now_v7();

        let result = s
            .create_rating(penilai, input(dinilai, "pelamar_ke_pemberi_kerja"))
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().bintang, 5);
    }

    #[tokio::test]
    async fn test_create_rating_given_lamaran_belum_selesai_when_submitted_then_returns_error() {
        let s = svc(false);
        let penilai = Uuid::now_v7();
        let dinilai = Uuid::now_v7();

        let result = s
            .create_rating(penilai, input(dinilai, "pemberi_kerja_ke_pelamar"))
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("belum berstatus Selesai"));
    }

    #[tokio::test]
    async fn test_create_rating_given_sudah_pernah_menilai_when_submitted_lagi_then_returns_error()
    {
        let s = svc(true);
        let penilai = Uuid::now_v7();
        let dinilai = Uuid::now_v7();
        let iklan_id = Uuid::now_v7();

        let first = CreateRatingInput {
            iklan_id,
            dinilai_id: dinilai,
            arah: "pelamar_ke_pemberi_kerja".to_string(),
            bintang: 5,
            ulasan: None,
        };
        s.create_rating(penilai, first).await.unwrap();

        // Pasangan (iklan_id, penilai, dinilai) PERSIS sama → harus ditolak.
        let duplicate = CreateRatingInput {
            iklan_id,
            dinilai_id: dinilai,
            arah: "pelamar_ke_pemberi_kerja".to_string(),
            bintang: 4,
            ulasan: None,
        };
        let result = s.create_rating(penilai, duplicate).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("sudah memberikan rating"));
    }

    #[tokio::test]
    async fn test_create_rating_given_diri_sendiri_when_submitted_then_returns_error() {
        let s = svc(true);
        let user = Uuid::now_v7();

        let result = s
            .create_rating(user, input(user, "pelamar_ke_pemberi_kerja"))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("diri sendiri"));
    }

    #[tokio::test]
    async fn test_get_aggregate_given_beberapa_rating_when_queried_then_returns_average() {
        let s = svc(true);
        let dinilai = Uuid::now_v7();
        let iklan_id = Uuid::now_v7();

        s.create_rating(
            Uuid::now_v7(),
            CreateRatingInput {
                iklan_id,
                dinilai_id: dinilai,
                arah: "pelamar_ke_pemberi_kerja".to_string(),
                bintang: 5,
                ulasan: None,
            },
        )
        .await
        .unwrap();
        s.create_rating(
            Uuid::now_v7(),
            CreateRatingInput {
                iklan_id: Uuid::now_v7(),
                dinilai_id: dinilai,
                arah: "pelamar_ke_pemberi_kerja".to_string(),
                bintang: 3,
                ulasan: None,
            },
        )
        .await
        .unwrap();

        let agg = s.get_aggregate(dinilai).await.unwrap();
        assert_eq!(agg.count, 2);
        assert_eq!(agg.average, 4.0);
    }

    #[tokio::test]
    async fn test_get_aggregate_given_belum_ada_rating_when_queried_then_returns_zero() {
        let s = svc(true);
        let agg = s.get_aggregate(Uuid::now_v7()).await.unwrap();
        assert_eq!(agg.count, 0);
        assert_eq!(agg.average, 0.0);
    }
}
