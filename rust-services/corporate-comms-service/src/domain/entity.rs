use chrono::{DateTime, Utc};
use corporate_comms_service_client::ArticleCategory;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CorporateArticle {
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
