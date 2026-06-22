//! # common-tracing — Tracing & OpenTelemetry setup
//!
//! ## Modes
//! - **Development** (APP_ENV=development): pretty logging ke stdout
//! - **Production** (APP_ENV=production): structured JSON logging ke stdout
//! - **OpenTelemetry** (OTEL_EXPORTER_OTLP_ENDPOINT diset): export tracing spans
//!   ke Grafana Cloud (OTLP gRPC) via `tracing-opentelemetry` bridge
//!
//! ## OpenTelemetry API Reference (v0.28)
//! - `opentelemetry_otlp::SpanExporter::builder().with_tonic()` — build OTLP exporter
//! - `opentelemetry_sdk::trace::SdkTracerProvider::builder().with_batch_exporter(exporter)` — tracer provider
//! - `opentelemetry_sdk::Resource::builder_empty().with_attribute(...)` — resource

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::{Timestamp, Uuid};

pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

// ── Standard tracing init (development / production JSON) ───────────────────

/// Inisialisasi tracing subscriber.
///
/// - Jika `OTEL_EXPORTER_OTLP_ENDPOINT` diset, function ini tidak melakukan
///   apa-apa karena OTel sudah menangani tracing-subscriber sendiri.
/// - Jika `APP_ENV=production`: JSON logging
/// - Jika `APP_ENV=development` atau tidak diset: pretty logging
pub fn init_tracing() {
    // Jika OTel active, jangan double init tracing-subscriber
    if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
        return;
    }

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

// ── OpenTelemetry (feature-gated: "otel") ───────────────────────────────────

/// OpenTelemetry tracer handle untuk shutdown.
#[cfg(feature = "otel")]
pub struct OtelHandle {
    tracer_provider: opentelemetry_sdk::trace::SdkTracerProvider,
}

#[cfg(feature = "otel")]
impl OtelHandle {
    /// Shutdown tracer provider (flush remaining spans).
    pub async fn shutdown(self) {
        tracing::info!("shutting down OpenTelemetry tracer provider");
        if let Err(e) = self.tracer_provider.shutdown() {
            tracing::warn!("OTel shutdown error: {e}");
        }
    }
}

/// Inisialisasi OpenTelemetry tracer dengan OTLP exporter ke Grafana Cloud.
///
/// Membaca env vars:
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP gRPC endpoint (wajib)
/// - `OTEL_EXPORTER_OTLP_HEADERS` — Authorization header (wajib)
/// - `OTEL_SERVICE_NAME` — Nama service (default: "rejki-backend")
///
/// Returns `Some(OtelHandle)` jika sukses, `None` jika env var tidak tersedia.
#[cfg(feature = "otel")]
pub async fn init_otel() -> Option<OtelHandle> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    let headers_raw = std::env::var("OTEL_EXPORTER_OTLP_HEADERS").ok()?;
    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "rejki-backend".into());

    // Parse headers dari format "key=value"
    let mut metadata = tonic::metadata::MetadataMap::new();
    for pair in headers_raw.split(',') {
        if let Some((k, v)) = pair.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            if let Ok(key) = key.parse::<tonic::metadata::AsciiMetadataKey>() {
                metadata.insert(key, val.parse().unwrap());
            }
        }
    }

    // ── Build OTLP SpanExporter via tonic ────────────────────────────────
    use opentelemetry_otlp::WithExportConfig; // trait for .with_endpoint()
    use opentelemetry_otlp::WithTonicConfig; // trait for .with_metadata()
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_metadata(metadata)
        .build()
        .ok()?;

    // ── Build Resource ───────────────────────────────────────────────────
    let resource = opentelemetry_sdk::Resource::builder_empty()
        .with_attribute(opentelemetry::KeyValue::new(
            "service.name",
            service_name.clone(),
        ))
        .with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_attribute(opentelemetry::KeyValue::new(
            "deployment.environment",
            std::env::var("APP_ENV").unwrap_or_else(|_| "unknown".into()),
        ))
        .build();

    // ── Build TracerProvider with batch exporter ─────────────────────────
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .with_resource(resource)
        .build();

    // Set global tracer provider
    use opentelemetry::trace::TracerProvider; // trait for .tracer()
    let _ = opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    // ── Bridge tracing ke OpenTelemetry ─────────────────────────────────
    let tracer = tracer_provider.tracer("rejki-backend");
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let is_prod = std::env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    if is_prod {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .with(tracing_opentelemetry::layer().with_tracer(tracer))
            .init();
    }

    tracing::info!(
        service = %service_name,
        "OpenTelemetry initialized — exporting spans to Grafana Cloud"
    );

    Some(OtelHandle { tracer_provider })
}

/// Shutdown OpenTelemetry tracer (convenience wrapper).
/// Jika tidak pakai OtelHandle, bisa panggil ini langsung.
#[cfg(feature = "otel")]
pub async fn shutdown_otel(handle: Option<OtelHandle>) {
    if let Some(h) = handle {
        h.shutdown().await;
    }
    // Force flush remaining spans
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
}

// ── Axum middleware — request_id_layer ──────────────────────────────────────

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
