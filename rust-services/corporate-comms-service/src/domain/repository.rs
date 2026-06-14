use super::entity::CorporateArticle;
use corporate_comms_service_client::ArticleCategory;
use uuid::Uuid;

/// Default page size untuk listing.
pub const DEFAULT_LIMIT: i64 = 20;

/// Params untuk `create()` — grouping untuk menghindari too_many_arguments.
pub struct CreateArticleParams<'a> {
    pub author_id: Uuid,
    pub title: &'a str,
    pub body: &'a str,
    pub category: &'a str,
    pub photo_object_key: Option<&'a str>,
}

/// Params untuk `update()` — grouping untuk menghindari too_many_arguments.
pub struct UpdateArticleParams<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub body: &'a str,
    pub category: &'a str,
    pub photo_object_key: Option<&'a str>,
}

/// Hasil listing terpaginasi.
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/// Params untuk `list()` — search by title, filter/sort by category, pagination.
pub struct ArticleListParams {
    pub q: Option<String>,
    pub category: Option<ArticleCategory>,
    pub sort_dir: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[allow(async_fn_in_trait)]
pub trait CorporateArticleRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<CorporateArticle>, anyhow::Error>;
    async fn list(
        &self,
        params: ArticleListParams,
    ) -> Result<ListResult<CorporateArticle>, anyhow::Error>;
    async fn create(
        &self,
        params: CreateArticleParams<'_>,
    ) -> Result<CorporateArticle, anyhow::Error>;
    async fn update(
        &self,
        params: UpdateArticleParams<'_>,
    ) -> Result<Option<CorporateArticle>, anyhow::Error>;
    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error>;
}
