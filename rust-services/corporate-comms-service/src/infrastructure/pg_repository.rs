use std::time::Instant;

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::entity::CorporateArticle;
use crate::domain::repository::{
    ArticleListParams, CorporateArticleRepository, CreateArticleParams, ListResult,
    UpdateArticleParams,
};
use corporate_comms_service_client::ArticleCategory;

// Macro expand ke literal string — memenuhi sqlx 0.9 SqlSafeStr (&'static str).
macro_rules! article_cols {
    () => {
        "id,author_id,category,title,body,photo_object_key,deleted_at,created_at,updated_at"
    };
}

pub struct PgCorporateArticleRepository {
    pool: PgPool,
}

impl PgCorporateArticleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Row mapper ────────────────────────────────────────────────────────────

fn row_to_article(r: &sqlx::postgres::PgRow) -> CorporateArticle {
    CorporateArticle {
        id: r.get("id"),
        author_id: r.get("author_id"),
        category: ArticleCategory::parse(&r.get::<String, _>("category")).unwrap_or_default(),
        title: r.get("title"),
        body: r.get("body"),
        photo_object_key: r.get("photo_object_key"),
        deleted_at: r.get("deleted_at"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

impl CorporateArticleRepository for PgCorporateArticleRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<CorporateArticle>, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "SELECT ",
            article_cols!(),
            " FROM comms.corporate_article WHERE id = $1"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| row_to_article(&r));
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: find_by_id"
            );
        }
        Ok(result)
    }

    async fn list(
        &self,
        params: ArticleListParams,
    ) -> Result<ListResult<CorporateArticle>, anyhow::Error> {
        let start = Instant::now();

        // Gunakan QueryBuilder untuk query dinamis (sqlx 0.9 safe).
        let mut count_builder = sqlx::QueryBuilder::new(
            "SELECT COUNT(*)::bigint FROM comms.corporate_article WHERE deleted_at IS NULL",
        );
        let mut data_builder = sqlx::QueryBuilder::new(concat!(
            "SELECT ",
            article_cols!(),
            " FROM comms.corporate_article WHERE deleted_at IS NULL"
        ));

        // Optional: search by title (ILIKE)
        if let Some(ref q) = params.q {
            if !q.trim().is_empty() {
                let pattern = format!("%{}%", q.trim());
                count_builder.push(" AND title ILIKE ");
                count_builder.push_bind(pattern.clone());
                data_builder.push(" AND title ILIKE ");
                data_builder.push_bind(pattern);
            }
        }

        // Optional: filter by category
        if let Some(ref cat) = params.category {
            count_builder.push(" AND category = ");
            count_builder.push_bind(cat.as_str());
            data_builder.push(" AND category = ");
            data_builder.push_bind(cat.as_str());
        }

        // Total
        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&self.pool)
            .await?;

        // Sort
        let sort_dir = params.sort_dir.as_deref().unwrap_or("asc");
        if sort_dir.eq_ignore_ascii_case("desc") {
            data_builder.push(" ORDER BY category DESC, created_at DESC");
        } else {
            data_builder.push(" ORDER BY category ASC, created_at DESC");
        }

        // Pagination
        data_builder.push(" LIMIT ");
        data_builder.push_bind(params.limit);
        data_builder.push(" OFFSET ");
        data_builder.push_bind(params.offset);

        let items: Vec<CorporateArticle> = data_builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(row_to_article)
            .collect();

        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: list"
            );
        }
        Ok(ListResult { items, total })
    }

    async fn create(
        &self,
        params: CreateArticleParams<'_>,
    ) -> Result<CorporateArticle, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "INSERT INTO comms.corporate_article (",
            article_cols!(),
            ") VALUES ($1,$2,$3,$4,$5,$6,NULL,now(),now()) RETURNING ",
            article_cols!()
        ))
        .bind(Uuid::now_v7())
        .bind(params.author_id)
        .bind(params.category)
        .bind(params.title)
        .bind(params.body)
        .bind(params.photo_object_key)
        .fetch_one(&self.pool)
        .await
        .map(|r| row_to_article(&r))?;
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: create"
            );
        }
        Ok(result)
    }

    async fn update(
        &self,
        params: UpdateArticleParams<'_>,
    ) -> Result<Option<CorporateArticle>, anyhow::Error> {
        let start = Instant::now();
        let result = sqlx::query(concat!(
            "UPDATE comms.corporate_article SET title=$1, body=$2, category=$3, photo_object_key=$4, updated_at=now() \
             WHERE id=$5 AND deleted_at IS NULL RETURNING ",
            article_cols!()
        ))
        .bind(params.title)
        .bind(params.body)
        .bind(params.category)
        .bind(params.photo_object_key)
        .bind(params.id)
        .fetch_optional(&self.pool)
        .await?
        .map(|r| row_to_article(&r));
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: update"
            );
        }
        Ok(result)
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, anyhow::Error> {
        let start = Instant::now();
        let affected = sqlx::query(
            "UPDATE comms.corporate_article SET deleted_at=now() WHERE id=$1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if start.elapsed().as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = start.elapsed().as_millis(),
                "slow DB query: soft_delete"
            );
        }
        Ok(affected > 0)
    }
}
