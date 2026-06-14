use storage_service_client::{FileInfo, StorageClient, StorageClientError, UploadPermission};

use crate::infrastructure::minio::MinioStorage;

pub struct StorageInProcessClient {
    storage: Option<MinioStorage>,
}

impl StorageInProcessClient {
    pub async fn new() -> Self {
        Self {
            storage: MinioStorage::from_env().await,
        }
    }
}

#[async_trait::async_trait]
impl StorageClient for StorageInProcessClient {
    async fn request_upload(
        &self,
        category: &str,
        user_id: uuid::Uuid,
        info: FileInfo,
    ) -> Result<UploadPermission, StorageClientError> {
        let (max_bytes, allowed_mimes) = match category {
            "avatar" => (5 * 1024 * 1024, &["image/jpeg", "image/png"][..]),
            "ktp" | "selfie" => (10 * 1024 * 1024, &["image/jpeg", "image/png"][..]),
            // Bukti suspend akun (US-07 / Q1): gambar atau PDF, maks 5MB per dokumen.
            "suspension-evidence" => (
                5 * 1024 * 1024,
                &["image/jpeg", "image/png", "application/pdf"][..],
            ),
            // Bukti suspend iklan: gambar atau PDF, maks 5MB per dokumen.
            // Ref: openspec/changes/extend-iklan-moderation/design.md D2
            "iklan-suspension-evidence" => (
                5 * 1024 * 1024,
                &["image/jpeg", "image/png", "application/pdf"][..],
            ),
            // Bukti transfer pelatihan: gambar atau PDF, maks 5MB.
            // Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D5
            "training-transfer-evidence" => (
                5 * 1024 * 1024,
                &["image/jpeg", "image/png", "application/pdf"][..],
            ),
            // Sertifikat pelatihan: gambar atau PDF, maks 10MB.
            // Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D5
            "training-certificate" => (
                10 * 1024 * 1024,
                &["image/jpeg", "image/png", "application/pdf"][..],
            ),
            // Foto artikel corporate communication: JPEG/PNG, maks 5MB.
            // Ref: openspec/changes/add-corporate-comms/design.md D5
            "article-photo" => (5 * 1024 * 1024, &["image/jpeg", "image/png"][..]),
            // Bukti aduan (report): JPEG/PNG/PDF, maks 5MB.
            // Ref: openspec/changes/add-content-reports/design.md D5
            "report-evidence" => (
                5 * 1024 * 1024,
                &["image/jpeg", "image/png", "application/pdf"][..],
            ),
            _ => return Err(StorageClientError::InvalidMime),
        };

        storage_service_client::validate_file(category, &info, max_bytes, allowed_mimes)?;

        let ext = match info.mime.as_str() {
            "image/png" => "png",
            "application/pdf" => "pdf",
            _ => "jpg",
        };
        let object_key = format!(
            "uploads/{}/{}/{}.{}",
            category,
            user_id,
            uuid::Uuid::now_v7(),
            ext
        );

        let presigned_url = match self.storage.as_ref() {
            Some(s) => s
                .presigned_upload(&object_key)
                .await
                .ok_or(StorageClientError::Unavailable)?,
            None => return Err(StorageClientError::Unavailable),
        };

        Ok(UploadPermission {
            presigned_url,
            object_key,
        })
    }

    async fn request_download(&self, object_key: &str) -> Result<String, StorageClientError> {
        match self.storage.as_ref() {
            Some(s) => s
                .presigned_download(object_key)
                .await
                .ok_or(StorageClientError::Unavailable),
            None => Err(StorageClientError::Unavailable),
        }
    }

    async fn delete(&self, object_key: &str) -> Result<(), StorageClientError> {
        match self.storage.as_ref() {
            Some(s) if s.delete_object(object_key).await => Ok(()),
            Some(_) => Err(StorageClientError::Unavailable),
            None => Err(StorageClientError::Unavailable),
        }
    }
}
