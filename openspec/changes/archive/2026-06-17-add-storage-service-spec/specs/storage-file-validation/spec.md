## ADDED Requirements

### Requirement: Validasi magic bytes untuk mencegah spoof MIME

Sistem SHALL menyediakan fungsi `verify_magic_bytes(mime, head)` yang memverifikasi byte awal berkas (magic bytes) cocok dengan MIME yang diklaim. Sistem SHALL mendukung verifikasi untuk JPEG (header `FF D8 FF`), PNG (header `89 50 4E 47 0D 0A 1A 0A`), dan PDF (header `%PDF-`).

#### Scenario: Magic bytes JPEG valid
- **WHEN** `verify_magic_bytes` dipanggil dengan MIME `image/jpeg` dan header yang dimulai `FF D8 FF`
- **THEN** sistem mengembalikan `Ok(())`

#### Scenario: Magic bytes PNG valid
- **WHEN** `verify_magic_bytes` dipanggil dengan MIME `image/png` dan header yang dimulai `89 50 4E 47 0D 0A 1A 0A`
- **THEN** sistem mengembalikan `Ok(())`

#### Scenario: Magic bytes PDF valid
- **WHEN** `verify_magic_bytes` dipanggil dengan MIME `application/pdf` dan header yang dimulai `%PDF-`
- **THEN** sistem mengembalikan `Ok(())`

#### Scenario: Magic bytes tidak cocok dengan MIME
- **WHEN** `verify_magic_bytes` dipanggil dengan MIME `image/jpeg` tetapi header adalah magic bytes PNG
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`

#### Scenario: MIME tidak didukung
- **WHEN** `verify_magic_bytes` dipanggil dengan MIME yang tidak dikenal (mis. `application/x-msdownload`)
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`

#### Scenario: Header terlalu pendek
- **WHEN** `verify_magic_bytes` dipanggil dengan array byte yang lebih pendek dari magic bytes yang diharapkan
- **THEN** sistem menolak (tidak panic)

### Requirement: Validasi ukuran dan MIME per kategori

Sistem SHALL menyediakan fungsi `validate_file(category, info, max_bytes, allowed_mimes)` untuk validasi ukuran dan MIME berdasarkan batas per kategori. Fungsi ini dipanggil oleh `StorageInProcessClient` sebelum menerbitkan presigned URL.

#### Scenario: Ukuran dalam batas dan MIME diizinkan
- **WHEN** `validate_file` dipanggil dengan `size_bytes ≤ max_bytes` dan MIME termasuk dalam `allowed_mimes`
- **THEN** sistem mengembalikan `Ok(())`

#### Scenario: Ukuran melebihi batas
- **WHEN** `validate_file` dipanggil dengan `size_bytes > max_bytes`
- **THEN** sistem menolak dengan `StorageClientError::FileTooLarge`

#### Scenario: MIME tidak diizinkan
- **WHEN** `validate_file` dipanggil dengan MIME yang tidak termasuk dalam `allowed_mimes`
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`
