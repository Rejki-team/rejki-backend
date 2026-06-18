//! Global rate limit middleware — defense-in-depth untuk semua endpoint.
//!
//! Diterapkan sebagai axum middleware di rejki-app SEBELUM service router.
//! Key: `rl:global:ip:{client_ip}`, 100 req/menit per IP.
//! Skip: `GET /health`.
//!
//! Ref: design doc D7, security-baseline.html §Rate Limiting.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use common_rate_limit::RateLimiter;
use std::sync::Arc;

/// Fallback IP bila tidak bisa diekstrak dari header.
const FALLBACK_IP: &str = "unknown";

/// Global rate limit: 100 req per menit per IP.
const GLOBAL_MAX_REQUESTS: i64 = 100;
const GLOBAL_WINDOW_SECS: i64 = 60;
const GLOBAL_KEY_PREFIX: &str = "rl:global:ip";

/// State yang di-inject ke middleware.
#[derive(Clone)]
pub struct RateLimitState {
    pub limiter: Option<Arc<dyn RateLimiter>>,
}

/// Ekstrak client IP dari request header (urutan prioritas):
/// 1. `X-Forwarded-For` (ambil IP pertama bila ada multiple)
/// 2. `X-Real-IP`
fn extract_client_ip(req: &Request) -> String {
    // X-Forwarded-For: bisa berisi daftar IP dipisah koma — ambil yang pertama
    if let Some(ff) = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        let first = ff.split(',').next().unwrap_or("").trim();
        if !first.is_empty() {
            return first.to_string();
        }
    }

    // X-Real-IP: single IP dari reverse proxy (nginx)
    if let Some(real) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let trimmed = real.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    FALLBACK_IP.to_string()
}

/// Middleware function — dipanggil untuk setiap request.
pub async fn global_rate_limit(
    State(state): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip health check
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    let rate_limiter = match &state.limiter {
        Some(rl) => rl.clone(),
        None => {
            // Rate limiter tidak terpasang — allow semua request (fail-open)
            return Ok(next.run(req).await);
        }
    };

    let ip = extract_client_ip(&req);
    let key = format!("{GLOBAL_KEY_PREFIX}:{ip}");

    if !rate_limiter
        .allow_raw(&key, GLOBAL_MAX_REQUESTS, GLOBAL_WINDOW_SECS)
        .await
    {
        let body = serde_json::json!({
            "success": false,
            "error": {
                "code": "RATE_LIMITED",
                "message": "Terlalu banyak permintaan. Silakan coba lagi dalam 60 detik."
            }
        });
        let resp = Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", "60")
            .header("Content-Type", "application/json; charset=utf-8")
            .body(axum::body::Body::from(
                serde_json::to_string(&body).unwrap(),
            ))
            .unwrap();
        return Ok(resp);
    }

    Ok(next.run(req).await)
}
