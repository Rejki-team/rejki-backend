use std::net::IpAddr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::domain::audit_log::AuditContext;

/// Extractor untuk AuditContext — hanya consume header (tidak body).
/// Aman digunakan bersamaan dengan ValidatedJson / Json.
impl<S: Send + Sync> FromRequestParts<S> for AuditContext {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip_address = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next()?.trim().parse::<IpAddr>().ok());

        let user_agent = parts
            .headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        let request_id = parts
            .headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned());

        Ok(AuditContext {
            ip_address,
            user_agent,
            request_id,
        })
    }
}
