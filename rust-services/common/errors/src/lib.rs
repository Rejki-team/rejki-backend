use axum::body::Body;
use axum::extract::{FromRequest, Request};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use uuid::{Timestamp, Uuid};
use validator::Validate;

// ── AppError ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound(String),

    #[error("gone")]
    Gone(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("validation error")]
    Validation(String),

    #[error("conflict")]
    Conflict(String),

    #[error("forbidden")]
    Forbidden(String),

    #[error("too many requests")]
    TooManyRequests(String),

    /// Rate limiting active — identical to TooManyRequests but with explicit
    /// error code RATE_LIMITED (per api-standard + security-baseline docs).
    #[error("rate limited")]
    RateLimited(String),

    /// Akun belum aktif (gating fitur). Kode mesin spesifik: ACCOUNT_NOT_ACTIVE.
    #[error("account not active")]
    AccountNotActive,

    /// Bukan admin — akses admin ditolak. Kode mesin spesifik: ACCOUNT_NOT_ADMIN.
    #[error("account not admin")]
    AccountNotAdmin,

    /// Role tidak mencukupi untuk mengakses endpoint ini. Kode mesin: INSUFFICIENT_ROLE.
    #[error("insufficient role")]
    InsufficientRole,

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m),
            AppError::Gone(m) => (StatusCode::GONE, "GONE", m),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "unauthorized".into(),
            ),
            AppError::Validation(m) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", m),
            AppError::Conflict(m) => (StatusCode::CONFLICT, "CONFLICT", m),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, "FORBIDDEN", m),
            AppError::TooManyRequests(m) => {
                let mut resp = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorBody {
                        error: "TOO_MANY_REQUESTS",
                        message: m,
                    }),
                )
                    .into_response();
                resp.headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
                return resp;
            }
            AppError::RateLimited(m) => {
                let mut resp = (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorBody {
                        error: "RATE_LIMITED",
                        message: m,
                    }),
                )
                    .into_response();
                resp.headers_mut()
                    .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
                return resp;
            }
            AppError::AccountNotActive => (
                StatusCode::FORBIDDEN,
                "ACCOUNT_NOT_ACTIVE",
                "akun belum aktif — lengkapi verifikasi untuk mengakses fitur".into(),
            ),
            AppError::AccountNotAdmin => (
                StatusCode::FORBIDDEN,
                "ACCOUNT_NOT_ADMIN",
                "akses admin diperlukan — akun tidak memiliki peran admin".into(),
            ),
            AppError::InsufficientRole => (
                StatusCode::FORBIDDEN,
                "INSUFFICIENT_ROLE",
                "peran anda tidak memiliki akses ke sumber daya ini".into(),
            ),
            AppError::Internal(e) => {
                tracing::error!(error = ?e, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "internal server error".into(),
                )
            }
        };
        (
            status,
            Json(ErrorBody {
                error: code,
                message,
            }),
        )
            .into_response()
    }
}

// ── ApiResponse<T> ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    pub request_id: String,
}

fn new_request_id() -> String {
    let ts = Timestamp::now(uuid::timestamp::context::NoContext);
    Uuid::new_v7(ts).to_string()
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data,
            meta: None,
            request_id: new_request_id(),
        }
    }

    pub fn with_meta(data: T, meta: impl Serialize) -> Self {
        Self {
            success: true,
            data,
            meta: Some(serde_json::to_value(meta).unwrap_or(serde_json::Value::Null)),
            request_id: new_request_id(),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

// ── Pagination meta types ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PaginatedMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: i64,
    pub total_pages: u32,
}

impl PaginatedMeta {
    pub fn new(page: u32, per_page: u32, total: i64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            page,
            per_page,
            total,
            total_pages,
        }
    }
}

#[derive(Serialize)]
pub struct CursorMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

// ── ValidatedJson<T> — extractor dengan validasi otomatis ─────────────────────

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: for<'de> Deserialize<'de> + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S>,
    <Json<T> as FromRequest<S>>::Rejection: std::fmt::Display,
{
    type Rejection = Response;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            let body = ErrorBody {
                error: "PARSE_ERROR",
                message: e.to_string(),
            };
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        })?;

        value.validate().map_err(|e| {
            let message = e
                .field_errors()
                .into_iter()
                .map(|(field, errs)| {
                    let msg = errs
                        .iter()
                        .filter_map(|ve| ve.message.as_ref().map(|m| m.as_ref()))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{field}: {msg}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            let body = ErrorBody {
                error: "VALIDATION_ERROR",
                message,
            };
            (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
        })?;

        Ok(ValidatedJson(value))
    }
}

// ── Location header helper (201 Created) ─────────────────────────────────────

pub fn created_response<T: Serialize>(body: ApiResponse<T>, location: &str) -> Response {
    let mut resp = (StatusCode::CREATED, Json(body)).into_response();
    if let Ok(val) = HeaderValue::from_str(location) {
        resp.headers_mut().insert(header::LOCATION, val);
    }
    resp
}

/// Tambahkan `Deprecation` dan `Sunset` header pada endpoint yang sudah deprecated.
pub fn deprecated_headers(resp: &mut Response) {
    resp.headers_mut()
        .insert("Deprecation", HeaderValue::from_static("true"));
    resp.headers_mut().insert(
        "Sunset",
        HeaderValue::from_static("Sat, 31 Dec 2026 23:59:59 GMT"),
    );
}
