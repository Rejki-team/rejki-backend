use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::{Timestamp, Uuid};

pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let is_prod = std::env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    if is_prod {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init();
    }
}

/// Axum middleware — inject `x-request-id` ke request dan response.
pub async fn request_id_layer(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| {
            let ts = Timestamp::now(uuid::timestamp::context::NoContext);
            Uuid::new_v7(ts).to_string()
        });

    if let Ok(val) = HeaderValue::from_str(&request_id) {
        req.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    // Gunakan Instrument agar span tidak menahan guard non-Send melintasi await
    // (guard `.entered()` membuat future menjadi !Send sehingga gagal sebagai Service).
    use tracing::Instrument;
    let span = tracing::info_span!("request", request_id = %request_id);
    let mut response = next.run(req).instrument(span).await;

    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    response
}
