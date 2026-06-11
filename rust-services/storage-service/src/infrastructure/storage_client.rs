use storage_service_client::{
    FileInfo, StorageClient, StorageClientError, UploadPermission,
};

use crate::infrastructure::minio::MinioStorage;

/// Implementasi StorageClient untuk mode in-process (Modular Monolith).
/// MinioStorage diinisialisasi dari env (async), boleh None bila env tidak diset.
pub struct StorageInProcessClient {
    storage: Option<MinioStorage>,
}

impl StorageInProcessClient {
    pub async fn new() -> Self {
        Self { storage: MinioStorage::from_env().await }
    }
}

#[async_trait::async_trait]
impl StorageClient for StorageInProcessClient {
    async fn request_upload(
        &self,
        category: &str,
        user_id:   uuid::Uuid,
        info:      FileInfo,
    ) -> Result<UploadPermission, StorageClientError> {
        let (max_bytes, allowed_mimes) = match category {
            "avatar"    => (5 * 1024 * 1024, &["image/jpeg", "image/png"][..]),
            "ktp" | "selfie" => (10 * 1024 * 1024, &["image/jpeg", "image/png"][..]),
            _ => return Err(StorageClientError::InvalidMime),
        };

        storage_service_client::validate_file(category, &info, max_bytes, allowed_mimes)?;

        let ext = if info.mime == "image/png" { "png" } else { "jpg" };
        let object_key = format!(
            "uploads/{}/{}/{}.{}",
            category, user_id, uuid::Uuid::now_v7(), ext
        );

        let storage_clone = self.storage.clone();
        let obj_key = object_key.clone();
        let presigned_url = storage_clone
            .as_ref()
            .and_then(|s| {
                // Blocking: presigned URL via AWS SDK is async, but this trait is sync.
                // Use tokio::runtime::Handle::current().block_on for simplicity.
                tokio::runtime::Handle::try_current()
                    .ok()
                    .and_then(|rt| rt.block_on(s.presigned_upload(&obj_key)))
            })
            .ok_or(StorageClientError::Unavailable)?;

        Ok(UploadPermission { presigned_url, object_key })
    }

    async fn request_download(
        &self,
        object_key: &str,
    ) -> Result<String, StorageClientError> {
        self.storage
            .as_ref()
            .and_then(|s| {
                tokio::runtime::Handle::try_current()
                    .ok()
                    .and_then(|rt| rt.block_on(s.presigned_download(object_key)))
            })
            .ok_or(StorageClientError::Unavailable)
    }
}
