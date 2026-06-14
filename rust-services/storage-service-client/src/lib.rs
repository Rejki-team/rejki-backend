/// Informasi berkas yang akan diunggah — dikirim klien saat minta presigned URL.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub mime: String,
    pub size_bytes: u64,
}

/// Izin unggah — dikembalikan ke klien setelah validasi.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadPermission {
    pub presigned_url: String,
    pub object_key: String,
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
pub fn validate_file(
    category: &str,
    info: &FileInfo,
    max_bytes: u64,
    allowed_mimes: &[&str],
) -> Result<(), StorageClientError> {
    if info.size_bytes > max_bytes {
        return Err(StorageClientError::FileTooLarge);
    }
    if !allowed_mimes.contains(&info.mime.as_str()) {
        return Err(StorageClientError::InvalidMime);
    }
    let _ = category;
    Ok(())
}

/// Verifikasi byte awal berkas (magic bytes) cocok dengan MIME yang diklaim.
/// Mencegah spoof ekstensi/mime: file `.jpg` palsu yang sebenarnya skrip akan ditolak.
///
/// Tanda tangan berkas (sumber: Gary Kessler File Signatures, file-recovery.com):
/// - JPEG (JFIF/Exif): `FF D8 FF`
/// - PNG:              `89 50 4E 47 0D 0A 1A 0A`
/// - PDF:              `25 50 44 46 2D` (`%PDF-`)
pub fn verify_magic_bytes(mime: &str, head: &[u8]) -> Result<(), StorageClientError> {
    let ok = match mime {
        "image/jpeg" => head.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => head.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        "application/pdf" => head.starts_with(b"%PDF-"),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(StorageClientError::InvalidMime)
    }
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
        user_id: uuid::Uuid,
        info: FileInfo,
    ) -> Result<UploadPermission, StorageClientError>;

    /// Minta presigned URL untuk download sementara (akses terbatas pemilik/admin).
    async fn request_download(&self, object_key: &str) -> Result<String, StorageClientError>;

    /// Hapus objek dari storage. Dipakai untuk pemusnahan dokumen (retensi K11).
    async fn delete(&self, object_key: &str) -> Result<(), StorageClientError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // JPEG: FF D8 FF (header JFIF/Exif).
    #[test]
    fn jpeg_valid_header_accepted() {
        let head = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(verify_magic_bytes("image/jpeg", &head).is_ok());
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A (8 byte penuh).
    #[test]
    fn png_valid_header_accepted() {
        let head = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
        assert!(verify_magic_bytes("image/png", &head).is_ok());
    }

    // PDF: %PDF-
    #[test]
    fn pdf_valid_header_accepted() {
        assert!(verify_magic_bytes("application/pdf", b"%PDF-1.7\n").is_ok());
    }

    // Berkas mengaku JPEG tapi byte-nya PNG → ditolak (anti-spoof).
    #[test]
    fn mismatched_header_rejected() {
        let png_head = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(matches!(
            verify_magic_bytes("image/jpeg", &png_head),
            Err(StorageClientError::InvalidMime)
        ));
    }

    // MIME tak dikenal → ditolak.
    #[test]
    fn unknown_mime_rejected() {
        assert!(matches!(
            verify_magic_bytes("application/x-msdownload", &[0x4D, 0x5A]),
            Err(StorageClientError::InvalidMime)
        ));
    }

    // Header terlalu pendek → ditolak (tidak panic).
    #[test]
    fn truncated_header_rejected() {
        assert!(verify_magic_bytes("image/png", &[0x89, 0x50]).is_err());
        assert!(verify_magic_bytes("image/jpeg", &[]).is_err());
    }

    // ── Article photo validation ────────────────────────────────────────

    #[test]
    fn article_photo_rejects_pdf() {
        let info = FileInfo {
            mime: "application/pdf".into(),
            size_bytes: 1_000_000,
        };
        assert!(matches!(
            validate_file(
                "article-photo",
                &info,
                5 * 1024 * 1024,
                &["image/jpeg", "image/png"]
            ),
            Err(StorageClientError::InvalidMime)
        ));
    }

    #[test]
    fn article_photo_rejects_oversize() {
        let info = FileInfo {
            mime: "image/jpeg".into(),
            size_bytes: 6 * 1024 * 1024, // > 5MB
        };
        assert!(matches!(
            validate_file(
                "article-photo",
                &info,
                5 * 1024 * 1024,
                &["image/jpeg", "image/png"]
            ),
            Err(StorageClientError::FileTooLarge)
        ));
    }

    #[test]
    fn article_photo_accepts_valid() {
        let info = FileInfo {
            mime: "image/png".into(),
            size_bytes: 1_000_000,
        };
        assert!(validate_file(
            "article-photo",
            &info,
            5 * 1024 * 1024,
            &["image/jpeg", "image/png"]
        )
        .is_ok());
    }
}
