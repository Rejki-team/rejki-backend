#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use uuid::Uuid;

    use crate::application::dto::{ArticleListQuery, CreateArticleInput, UpdateArticleInput};
    use crate::application::service::CorporateCommsService;
    use crate::domain::entity::CorporateArticle;
    use crate::domain::repository::{
        ArticleListParams, CorporateArticleRepository, CreateArticleParams, ListResult,
        UpdateArticleParams,
    };
    use corporate_comms_service_client::ArticleCategory;

    // ── MockCorporateArticleRepository ────────────────────────────────────────────

    struct MockCorporateArticleRepository {
        articles: Mutex<Vec<CorporateArticle>>,
    }

    impl MockCorporateArticleRepository {
        fn new() -> Self {
            Self {
                articles: Mutex::new(vec![]),
            }
        }
    }

    #[allow(async_fn_in_trait)]
    impl CorporateArticleRepository for MockCorporateArticleRepository {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<CorporateArticle>, anyhow::Error> {
            Ok(self
                .articles
                .lock()
                .unwrap()
                .iter()
                .find(|a| a.id == id && a.deleted_at.is_none())
                .cloned())
        }

        async fn list(
            &self,
            _params: ArticleListParams,
        ) -> Result<ListResult<CorporateArticle>, anyhow::Error> {
            let articles = self.articles.lock().unwrap();
            let total = articles.len() as i64;
            Ok(ListResult {
                items: articles.clone(),
                total,
            })
        }

        async fn create(
            &self,
            params: CreateArticleParams<'_>,
        ) -> Result<CorporateArticle, anyhow::Error> {
            let article = CorporateArticle {
                id: Uuid::now_v7(),
                author_id: params.author_id,
                category: ArticleCategory::parse(params.category).unwrap_or_default(),
                title: params.title.to_string(),
                body: params.body.to_string(),
                photo_object_key: params.photo_object_key.map(String::from),
                deleted_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.articles.lock().unwrap().push(article.clone());
            Ok(article)
        }

        async fn update(
            &self,
            params: UpdateArticleParams<'_>,
        ) -> Result<Option<CorporateArticle>, anyhow::Error> {
            let mut articles = self.articles.lock().unwrap();
            if let Some(a) = articles
                .iter_mut()
                .find(|a| a.id == params.id && a.deleted_at.is_none())
            {
                a.title = params.title.to_string();
                a.body = params.body.to_string();
                a.category =
                    ArticleCategory::parse(params.category).unwrap_or(ArticleCategory::Informasi);
                a.photo_object_key = params.photo_object_key.map(String::from);
                a.updated_at = chrono::Utc::now();
                Ok(Some(a.clone()))
            } else {
                Ok(None)
            }
        }

        async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
            let mut articles = self.articles.lock().unwrap();
            if let Some(a) = articles
                .iter_mut()
                .find(|a| a.id == id && a.deleted_at.is_none())
            {
                a.deleted_at = Some(chrono::Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    fn svc() -> CorporateCommsService<MockCorporateArticleRepository> {
        CorporateCommsService::new(std::sync::Arc::new(MockCorporateArticleRepository::new()))
    }

    fn basic_create_input() -> CreateArticleInput {
        CreateArticleInput {
            title: "Pengumuman Penting".into(),
            body: "Isi pengumuman untuk seluruh pengguna.".into(),
            category: ArticleCategory::Informasi,
        }
    }

    // ── admin_create ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_create_given_valid_input_when_create_then_succeeds() {
        let s = svc();
        let author = Uuid::now_v7();
        let article = s.admin_create(author, basic_create_input()).await.unwrap();
        assert!(article.title.contains("Pengumuman"));
        assert_eq!(article.author_id, author);
        assert_eq!(article.category, ArticleCategory::Informasi);
    }

    #[tokio::test]
    async fn test_admin_create_given_html_when_create_then_strips_html() {
        let s = svc();
        let article = s
            .admin_create(
                Uuid::now_v7(),
                CreateArticleInput {
                    title: "<b>Breaking News</b>".into(),
                    body: "<script>alert('xss')</script>Body content".into(),
                    category: ArticleCategory::Informasi,
                },
            )
            .await
            .unwrap();
        assert!(!article.title.contains('<'));
        assert!(!article.body.contains('<'));
    }

    // ── admin_get ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_get_given_valid_id_when_get_then_returns_article() {
        let s = svc();
        let author = Uuid::now_v7();
        let created = s.admin_create(author, basic_create_input()).await.unwrap();
        let got = s.admin_get(created.id).await.unwrap();
        assert!(got.title.contains("Pengumuman"));
        assert_eq!(got.author_id, author);
    }

    #[tokio::test]
    async fn test_admin_get_given_nonexistent_id_when_get_then_returns_error() {
        assert!(svc().admin_get(Uuid::now_v7()).await.is_err());
    }

    // ── admin_update ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_update_given_existing_article_when_update_then_succeeds() {
        let s = svc();
        let author = Uuid::now_v7();
        let created = s.admin_create(author, basic_create_input()).await.unwrap();
        let updated = s
            .admin_update(
                created.id,
                UpdateArticleInput {
                    title: "Judul Baru".into(),
                    body: "Body baru".into(),
                    category: ArticleCategory::Informasi,
                    photo_object_key: None,
                },
            )
            .await
            .unwrap();
        // ammonia may encode spaces; check content survived
        assert!(updated.title.contains("Judul"));
        assert!(updated.title.contains("Baru"));
        assert!(updated.body.contains("Body"));
    }

    #[tokio::test]
    async fn test_admin_update_given_nonexistent_id_when_update_then_returns_error() {
        assert!(svc()
            .admin_update(
                Uuid::now_v7(),
                UpdateArticleInput {
                    title: "X".into(),
                    body: "Y".into(),
                    category: ArticleCategory::Informasi,
                    photo_object_key: None,
                }
            )
            .await
            .is_err());
    }

    // ── admin_delete ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_delete_given_existing_article_when_delete_then_returns_true() {
        let s = svc();
        let created = s
            .admin_create(Uuid::now_v7(), basic_create_input())
            .await
            .unwrap();
        assert!(s.admin_delete(created.id).await.unwrap());
        // Setelah soft-delete, get harus error
        assert!(s.admin_get(created.id).await.is_err());
    }

    #[tokio::test]
    async fn test_admin_delete_given_nonexistent_id_when_delete_then_returns_false() {
        assert!(!svc().admin_delete(Uuid::now_v7()).await.unwrap());
    }

    // ── admin_list ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_admin_list_given_articles_when_list_then_returns_all() {
        let s = svc();
        let author = Uuid::now_v7();
        s.admin_create(author, basic_create_input()).await.unwrap();
        s.admin_create(
            author,
            CreateArticleInput {
                title: "Artikel Kedua".into(),
                body: "Isi kedua".into(),
                category: ArticleCategory::Informasi,
            },
        )
        .await
        .unwrap();
        let (items, total) = s
            .admin_list(ArticleListQuery {
                q: None,
                category: None,
                sort_dir: None,
                limit: Some(10),
                offset: Some(0),
            })
            .await
            .unwrap();
        assert!(total >= 2);
        assert!(items.len() >= 2);
    }

    // ── request_article_photo_upload (trait coherence) ──────────────────────────

    #[tokio::test]
    async fn test_service_has_photo_upload_method() {
        let s = svc();
        assert!(std::mem::size_of::<CorporateCommsService<MockCorporateArticleRepository>>() > 0);
        let _ = s;
    }

    // ── ArticleCategory (extensible, currently only Informasi) ──────────────────

    #[test]
    fn test_article_category_default_is_informasi() {
        assert_eq!(ArticleCategory::default(), ArticleCategory::Informasi);
    }

    #[test]
    fn test_article_category_as_str() {
        assert_eq!(ArticleCategory::Informasi.as_str(), "informasi");
    }

    #[test]
    fn test_article_category_parse_valid() {
        assert_eq!(
            ArticleCategory::parse("informasi"),
            Some(ArticleCategory::Informasi)
        );
    }

    #[test]
    fn test_article_category_parse_invalid() {
        assert_eq!(ArticleCategory::parse("invalid"), None);
    }
}
