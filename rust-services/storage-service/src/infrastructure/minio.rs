//! Presigned URL generator via official AWS SDK (aws-sdk-s3), using
//! MinIO-compatible endpoint override. Zero external S3 dependencies beyond SDK.
//!
//! Konfigurasi env: MINIO_ENDPOINT, MINIO_ACCESS_KEY, MINIO_SECRET_KEY, MINIO_BUCKET,
//! AWS_REGION (default us-east-1).

use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

const UPLOAD_TTL_SECS: u64   = 600;
const DOWNLOAD_TTL_SECS: u64 = 300;

#[derive(Clone)]
pub struct MinioStorage {
    client: Client,
    bucket: String,
}

impl MinioStorage {
    pub async fn from_env() -> Option<Self> {
        let endpoint   = std::env::var("MINIO_ENDPOINT").ok()?;
        let access_key = std::env::var("MINIO_ACCESS_KEY").ok()?;
        let secret_key = std::env::var("MINIO_SECRET_KEY").ok()?;
        let bucket     = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "rejki-dokumen".into());
        let region     = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into());

        let creds = Credentials::new(&access_key, &secret_key, None, None, "minio");

        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .endpoint_url(&endpoint)
            .force_path_style(true)
            .credentials_provider(creds)
            .build();

        let client = Client::from_conf(config);
        Some(Self { client, bucket })
    }

    pub async fn presigned_upload(&self, object_key: &str) -> Option<String> {
        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(object_key)
            .presigned(PresigningConfig::expires_in(
                Duration::from_secs(UPLOAD_TTL_SECS),
            ).ok()?)
            .await
            .ok()?;

        Some(req.uri().to_string())
    }

    pub async fn presigned_download(&self, object_key: &str) -> Option<String> {
        let req = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(object_key)
            .presigned(PresigningConfig::expires_in(
                Duration::from_secs(DOWNLOAD_TTL_SECS),
            ).ok()?)
            .await
            .ok()?;

        Some(req.uri().to_string())
    }
}
