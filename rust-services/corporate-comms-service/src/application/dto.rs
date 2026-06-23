use chrono::{DateTime, Utc};
use corporate_comms_service_client::ArticleCategory;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ══════════════════════════════════════════════════════════════════════════
// Response
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct AdminArticleResponse {
    pub id: Uuid,
    pub author_id: Uuid,
    pub category: ArticleCategory,
    pub title: String,
    pub body: String,
    pub photo_object_key: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════════
// Inputs
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Validate)]
pub struct CreateArticleInput {
    #[validate(length(min = 3, max = 200))]
    pub title: String,
    #[validate(length(min = 1))]
    pub body: String,
    #[serde(default = "default_category")]
    pub category: ArticleCategory,
}

fn default_category() -> ArticleCategory {
    ArticleCategory::Informasi
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateArticleInput {
    #[validate(length(min = 3, max = 200))]
    pub title: String,
    #[validate(length(min = 1))]
    pub body: String,
    pub category: ArticleCategory,
    pub photo_object_key: Option<String>,
}

/// Input untuk meminta presigned upload foto artikel.
#[derive(Debug, Deserialize, Validate)]
pub struct ArticlePhotoInput {
    pub mime: String,
    #[validate(range(min = 1, max = 5242880))]
    pub size_bytes: u64,
}

// ══════════════════════════════════════════════════════════════════════════
// Query params
// ══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ArticleListQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
