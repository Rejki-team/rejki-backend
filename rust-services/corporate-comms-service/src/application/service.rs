use std::sync::Arc;
use uuid::Uuid;

use super::dto::{
    AdminArticleResponse, ArticleListQuery, ArticlePhotoInput, CreateArticleInput,
    UpdateArticleInput,
};
use crate::domain::entity::CorporateArticle;
use crate::domain::repository::{
    ArticleListParams, CorporateArticleRepository, CreateArticleParams, UpdateArticleParams,
    DEFAULT_LIMIT,
};
use common_rate_limit::RateLimiter;
use corporate_comms_service_client::ArticleCategory;
use storage_service_client::StorageClient;

/// Nama kategori storage — bukan hardcoded string literal.
pub mod storage_category {
    pub const ARTICLE_PHOTO: &str = "article-photo";
}

pub struct CorporateCommsService<R: CorporateArticleRepository> {
    repo: Arc<R>,
    rate_limiter: Option<Arc<dyn RateLimiter>>,
}

impl<R: CorporateArticleRepository> CorporateCommsService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self {
            repo,
            rate_limiter: None,
        }
    }
    pub fn with_rate_limiter(mut self, rl: Arc<dyn RateLimiter>) -> Self {
        self.rate_limiter = Some(rl);
        self
    }

    // ── Admin: list articles ─────────────────────────────────────────────

    pub async fn admin_list(
        &self,
        query: ArticleListQuery,
    ) -> Result<(Vec<AdminArticleResponse>, i64), anyhow::Error> {
        let category = query.category.as_deref().and_then(ArticleCategory::parse);
        let result = self
            .repo
            .list(ArticleListParams {
                q: query.q,
                category,
                sort_dir: query.sort_dir,
                limit: query.limit.unwrap_or(DEFAULT_LIMIT),
                offset: query.offset.unwrap_or(0),
            })
            .await?;
        Ok((
            result.items.into_iter().map(to_admin_resp).collect(),
            result.total,
        ))
    }

    // ── Admin: get by id ─────────────────────────────────────────────────

    pub async fn admin_get(&self, id: Uuid) -> Result<AdminArticleResponse, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .map(to_admin_resp)
            .ok_or_else(|| anyhow::anyhow!("artikel tidak ditemukan"))
    }

    // ── Admin: create article ────────────────────────────────────────────

    pub async fn admin_create(
        &self,
        author_id: Uuid,
        input: CreateArticleInput,
    ) -> Result<CorporateArticle, anyhow::Error> {
        // Rate limit: 10 req/15 menit per admin
        if let Some(rl) = &self.rate_limiter {
            if !rl
                .allow("comms:create_article", &author_id.to_string())
                .await
            {
                return Err(anyhow::anyhow!(
                    "terlalu banyak permintaan, coba lagi nanti"
                ));
            }
        }
        self.repo
            .create(CreateArticleParams {
                author_id,
                title: &sanitize(&input.title),
                body: &sanitize(&input.body),
                category: input.category.as_str(),
                photo_object_key: None,
            })
            .await
    }

    // ── Admin: update article ────────────────────────────────────────────

    pub async fn admin_update(
        &self,
        id: Uuid,
        input: UpdateArticleInput,
    ) -> Result<CorporateArticle, anyhow::Error> {
        let existing = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("artikel tidak ditemukan"))?;

        self.repo
            .update(UpdateArticleParams {
                id,
                title: &sanitize(&input.title),
                body: &sanitize(&input.body),
                category: input.category.as_str(),
                photo_object_key: input
                    .photo_object_key
                    .as_deref()
                    .or(existing.photo_object_key.as_deref()),
            })
            .await?
            .ok_or_else(|| anyhow::anyhow!("artikel tidak ditemukan"))
    }

    // ── Admin: soft-delete article ───────────────────────────────────────

    pub async fn admin_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        self.repo.soft_delete(id).await
    }

    // ── Photo upload ─────────────────────────────────────────────────────

    pub async fn request_article_photo_upload(
        &self,
        storage: &dyn StorageClient,
        user_id: Uuid,
        input: ArticlePhotoInput,
    ) -> Result<storage_service_client::UploadPermission, anyhow::Error> {
        storage
            .request_upload(
                storage_category::ARTICLE_PHOTO,
                user_id,
                storage_service_client::FileInfo {
                    mime: input.mime,
                    size_bytes: input.size_bytes,
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("storage error: {e}"))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

#[inline]
fn sanitize(s: &str) -> String {
    ammonia::clean_text(s)
}

// ── Mapper functions ──────────────────────────────────────────────────────

pub fn to_admin_resp(e: CorporateArticle) -> AdminArticleResponse {
    AdminArticleResponse {
        id: e.id,
        author_id: e.author_id,
        category: e.category,
        title: e.title,
        body: e.body,
        photo_object_key: e.photo_object_key,
        deleted_at: e.deleted_at,
        created_at: e.created_at,
        updated_at: e.updated_at,
    }
}
