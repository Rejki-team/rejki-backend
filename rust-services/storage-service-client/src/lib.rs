/// Informasi berkas yang akan diunggah — dikirim klien saat minta presigned URL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub mime:      String,
    pub size_bytes: u64,
}

/// Izin unggah — dikembalikan ke klien setelah validasi.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadPermission {
    pub presigned_url: String,
    pub object_key:    String,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageClientError {
    #[error("file too large")]
    FileTooLarge,
    #[error("invalid mime type")]
    InvalidMime,
    #[error("storage service unavailable")]
    Unavailable,
}

/// Validasi MIME & ukuran file berdasarkan kategori.
pub fn validate_file(category: &str, info: &FileInfo, max_bytes: u64, allowed_mimes: &[&str]) -> Result<(), StorageClientError> {
    if info.size_bytes > max_bytes {
        return Err(StorageClientError::FileTooLarge);
    }
    if !allowed_mimes.contains(&info.mime.as_str()) {
        return Err(StorageClientError::InvalidMime);
    }
    let _ = category;
    Ok(())
}

/// Kontrak publik untuk storage (object upload/download).
/// Implementasi di `storage-service` (MinIO SDK).
#[async_trait::async_trait]
pub trait StorageClient: Send + Sync {
    /// Minta presigned URL untuk upload. StorageService memvalidasi MIME & ukuran,
    /// lalu mengembalikan URL + object_key yang dibuat server.
    async fn request_upload(
        &self,
        category: &str,
        user_id:   uuid::Uuid,
        info:      FileInfo,
    ) -> Result<UploadPermission, StorageClientError>;

    /// Minta presigned URL untuk download sementara (akses terbatas pemilik/admin).
    async fn request_download(
        &self,
        object_key: &str,
    ) -> Result<String, StorageClientError>;
}
