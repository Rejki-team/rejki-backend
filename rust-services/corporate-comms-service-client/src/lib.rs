/// Kategori artikel corporate communication. Dirancang extensible.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArticleCategory {
    #[default]
    Informasi,
}

/// Nama kategori yang disimpan di DB — jangan hardcode string literal.
pub mod category_name {
    pub const INFORMASI: &str = "informasi";
}

impl ArticleCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArticleCategory::Informasi => category_name::INFORMASI,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            category_name::INFORMASI => Some(ArticleCategory::Informasi),
            _ => None,
        }
    }
}

impl std::fmt::Display for ArticleCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Ringkasan artikel — dipakai oleh domain lain (mis. mobile app).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArticleSummary {
    pub id: uuid::Uuid,
    pub title: String,
    pub category: ArticleCategory,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// `async fn` di trait kini stabil; lint hanya menyoroti ketiadaan Send bound otomatis.
// Trait ini dipakai in-process (single composition root), jadi cukup di-allow.
#[allow(async_fn_in_trait)]
pub trait CommsClient: Send + Sync {
    /// Cek apakah artikel dengan id tertentu ada (tidak soft-deleted).
    async fn article_exists(&self, id: uuid::Uuid) -> Result<bool, CommsClientError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CommsClientError {
    #[error("article not found")]
    NotFound,
    #[error("service unavailable")]
    Unavailable,
}
