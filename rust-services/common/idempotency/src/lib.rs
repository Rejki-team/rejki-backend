use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use common_errors::{ApiResponse, AppError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

// ── Trait ──────────────────────────────────────────────────────────────────

/// Penyimpanan idempotency key — memungkinkan implementasi in-memory untuk test.
#[async_trait::async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Coba lock idempotency key.
    /// - `None` → key baru, lock berhasil
    /// - `Some(CachedResponse)` → key sudah ada dengan hash sama → return response cached
    /// - `Err(IdempotencyConflict)` → key sudah ada dengan hash berbeda
    async fn try_lock(
        &self,
        key: &str,
        request_hash: &str,
    ) -> Result<Option<CachedResponse>, IdempotencyError>;

    /// Tandai key sebagai completed dengan response status + body.
    async fn complete(
        &self,
        key: &str,
        response_status: u16,
        response_body: &serde_json::Value,
    ) -> Result<(), anyhow::Error>;

    /// Bersihkan key yang expired (TTL > 24 jam).
    /// Dipanggil otomatis di `try_lock`.
    async fn cleanup_expired(&self) -> Result<(), anyhow::Error>;
}

// ── DTO ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum IdempotencyError {
    #[error("idempotency key sudah dipakai dengan request berbeda")]
    Conflict,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// ── PostgreSQL Implementation ──────────────────────────────────────────────

pub struct PgIdempotencyStore {
    pool: PgPool,
}

impl PgIdempotencyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Clone for PgIdempotencyStore {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

#[async_trait::async_trait]
impl IdempotencyStore for PgIdempotencyStore {
    async fn try_lock(
        &self,
        key: &str,
        request_hash: &str,
    ) -> Result<Option<CachedResponse>, IdempotencyError> {
        // Cleanup expired dulu
        self.cleanup_expired()
            .await
            .map_err(IdempotencyError::Other)?;

        // Coba INSERT — unique constraint akan gagal jika key sudah ada
        let insert_result = sqlx::query(
            "INSERT INTO auth.idempotency_keys (id, key, request_hash)
             VALUES (gen_random_uuid(), $1, $2)
             ON CONFLICT (key) DO NOTHING
             RETURNING request_hash",
        )
        .bind(key)
        .bind(request_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| IdempotencyError::Other(e.into()))?;

        // Jika INSERT berhasil → key baru, lock acquired
        if insert_result.is_some() {
            return Ok(None);
        }

        // Key sudah ada — ambil response yang tersimpan
        let row = sqlx::query(
            "SELECT request_hash, response_status, response_body
             FROM auth.idempotency_keys
             WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| IdempotencyError::Other(e.into()))?;

        match row {
            Some(r) => {
                use sqlx::Row;
                let existing_hash: String = r
                    .try_get("request_hash")
                    .map_err(|e| IdempotencyError::Other(e.into()))?;
                let response_status: Option<i32> = r
                    .try_get("response_status")
                    .map_err(|e| IdempotencyError::Other(e.into()))?;
                let response_body: Option<serde_json::Value> = r
                    .try_get("response_body")
                    .map_err(|e| IdempotencyError::Other(e.into()))?;

                // Cek apakah hash sama
                if existing_hash == request_hash {
                    // Hash cocok → return cached response
                    let cached = CachedResponse {
                        status: response_status.unwrap_or(200) as u16,
                        body: response_body.unwrap_or(serde_json::Value::Null),
                    };
                    Ok(Some(cached))
                } else {
                    // Hash berbeda → conflict
                    Err(IdempotencyError::Conflict)
                }
            }
            None => {
                // Key sudah dihapus (expired) — gap kecil antar cleanup dan SELECT
                // Coba INSERT lagi
                sqlx::query(
                    "INSERT INTO auth.idempotency_keys (id, key, request_hash)
                     VALUES (gen_random_uuid(), $1, $2)
                     ON CONFLICT (key) DO NOTHING",
                )
                .bind(key)
                .bind(request_hash)
                .execute(&self.pool)
                .await
                .map_err(|e| IdempotencyError::Other(e.into()))?;
                Ok(None)
            }
        }
    }

    async fn complete(
        &self,
        key: &str,
        response_status: u16,
        response_body: &serde_json::Value,
    ) -> Result<(), anyhow::Error> {
        let status_i32 = response_status as i32;
        sqlx::query(
            "UPDATE auth.idempotency_keys
             SET response_status = $2, response_body = $3
             WHERE key = $1",
        )
        .bind(key)
        .bind(status_i32)
        .bind(response_body)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<(), anyhow::Error> {
        sqlx::query(
            "DELETE FROM auth.idempotency_keys
             WHERE created_at < now() - interval '24 hours'",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ── Middleware ─────────────────────────────────────────────────────────────

/// State untuk idempotency middleware — di-inject via `from_fn_with_state`.
/// Generic S memungkinkan penggunaan `PgIdempotencyStore` atau mock test.
#[derive(Clone)]
pub struct IdempotencyState<S: IdempotencyStore> {
    pub store: Arc<S>,
}

/// Computes SHA-256 hex dari request body.
fn hash_body(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    hex::encode(hasher.finalize())
}

/// Axum middleware untuk Idempotency-Key.
///
/// Pasang per route via `route_layer` atau `layer`:
/// ```rust,ignore
/// .route_layer(middleware::from_fn_with_state(
///     IdempotencyState { store },
///     idempotency_middleware::<PgIdempotencyStore>,
/// ))
/// ```
pub async fn idempotency_middleware<S: IdempotencyStore + 'static>(
    State(state): State<IdempotencyState<S>>,
    mut req: Request,
    next: Next,
) -> Response {
    // 1. Cek header Idempotency-Key
    let idempotency_key = match req.headers().get("Idempotency-Key") {
        Some(val) => match val.to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => {
                return AppError::Validation("Idempotency-Key header tidak valid".into())
                    .into_response();
            }
        },
        None => {
            // Tidak ada header → bypass
            return next.run(req).await;
        }
    };

    // 2. Baca body untuk hash
    let body_bytes = match axum::body::to_bytes(std::mem::take(req.body_mut()), 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "gagal membaca request body untuk idempotency");
            return next.run(req).await;
        }
    };

    let request_hash = hash_body(&body_bytes);

    // 3. Coba lock idempotency key
    match state.store.try_lock(&idempotency_key, &request_hash).await {
        Ok(None) => {
            // Key baru — set body back, lanjut handler
            *req.body_mut() = Body::from(body_bytes.clone());
            let resp = next.run(req).await;

            // 4. Jika response sukses (2xx), cache response
            if resp.status().is_success() {
                let (parts, body) = resp.into_parts();
                let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
                    Ok(b) => b,
                    Err(_) => {
                        return Response::from_parts(parts, Body::from(body_bytes));
                    }
                };
                let status = parts.status.as_u16();

                // Parse body sebagai JSON
                if let Ok(json_body) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                    if let Err(e) = state
                        .store
                        .complete(&idempotency_key, status, &json_body)
                        .await
                    {
                        tracing::warn!(error = %e, "gagal menyimpan idempotency response");
                    }
                }

                return Response::from_parts(parts, Body::from(body_bytes));
            }

            resp
        }
        Ok(Some(cached)) => {
            // Key sudah ada dengan hash sama — return cached response
            let status = StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK);
            let resp = ApiResponse::<serde_json::Value> {
                success: status.is_success(),
                data: cached.body,
                meta: None,
                request_id: None,
            };
            (status, Json(resp)).into_response()
        }
        Err(IdempotencyError::Conflict) => {
            // Key sudah dipakai dengan request berbeda
            AppError::IdempotencyConflict(
                "Idempotency-Key sudah dipakai dengan payload berbeda".into(),
            )
            .into_response()
        }
        Err(IdempotencyError::Other(e)) => {
            tracing::warn!(error = %e, "idempotency store error — fail-open, lanjutkan request");
            *req.body_mut() = Body::from(body_bytes.clone());
            next.run(req).await
        }
    }
}
