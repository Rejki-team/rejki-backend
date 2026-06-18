## ADDED Requirements

### Requirement: Presigned URL download sementara

Sistem SHALL menyediakan presigned URL download sementara melalui `StorageClient::request_download(object_key)`. URL SHALL berlaku selama maksimal 5 menit (300 detik). URL SHALL bersifat non-publik — hanya diberikan ke pemilik dokumen atau admin terotorisasi oleh application layer.

#### Scenario: Download dengan object key valid
- **WHEN** klien terotorisasi meminta presigned URL download dengan object key yang ada di storage
- **THEN** sistem mengembalikan URL temporer yang dapat digunakan untuk mengunduh objek

#### Scenario: Object key tidak ada di storage
- **WHEN** klien meminta presigned URL download untuk object key yang tidak ada
- **THEN** sistem tetap mengembalikan URL (presigned URL tidak memvalidasi keberadaan objek — kegagalan hanya terjadi saat klien mengakses URL)

#### Scenario: Storage tidak tersedia
- **WHEN** MinIO endpoint tidak dikonfigurasi
- **THEN** sistem menolak dengan `StorageClientError::Unavailable`

### Requirement: URL download tidak dapat dibagikan permanen

Sistem SHALL menetapkan TTL presigned download maksimal 5 menit. Sistem SHALL TIDAK menyediakan mekanisme untuk URL publik permanen.

#### Scenario: URL kedaluwarsa setelah TTL
- **WHEN** klien mencoba mengakses presigned download URL setelah 5 menit
- **THEN** MinIO menolak akses (akses hanya melalui presigned URL baru via backend)
