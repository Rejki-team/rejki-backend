use serde::Deserialize;

/// MinIO (S3-compatible) configuration — semua field required.
#[derive(Debug, Clone, Deserialize)]
pub struct MinioConfig {
    /// MinIO endpoint URL, e.g., `http://minio:9000`.
    pub endpoint: String,
    /// Access key / username.
    pub access_key: String,
    /// Secret key / password.
    pub secret_key: String,
    /// Default bucket for document uploads.
    pub bucket: String,
    /// Backup bucket (optional, untuk DB backup).
    pub backup_bucket: Option<String>,
}
