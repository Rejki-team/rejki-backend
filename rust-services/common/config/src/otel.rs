use serde::Deserialize;

/// OpenTelemetry / Grafana Cloud configuration — semua optional.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct OtelConfig {
    /// OTLP gRPC endpoint, e.g., `https://otlp-grpc.{zone}.grafana.net:4317`.
    pub exporter_otlp_endpoint: Option<String>,
    /// Authorization header value, e.g., `Basic base64(instanceId:token)`.
    pub exporter_otlp_headers: Option<String>,
    /// Service name for traces (default: `rejki-backend`).
    pub service_name: String,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            exporter_otlp_endpoint: None,
            exporter_otlp_headers: None,
            service_name: "rejki-backend".into(),
        }
    }
}
