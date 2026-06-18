## ADDED Requirements

### Requirement: Presigned URL upload dengan validasi per kategori

Sistem SHALL menyediakan presigned URL upload ke MinIO (S3-compatible) melalui `StorageClient::request_upload(category, user_id, info)`. Sistem SHALL memvalidasi MIME dan ukuran berkas terhadap whitelist per kategori SEBELUM menerbitkan presigned URL. Object key SHALL dibuat server-side dengan format `uploads/{category}/{user_id}/{uuid}.{ext}` dan SHALL TIDAK dapat dipilih oleh klien.

#### Scenario: Upload dengan MIME dan ukuran valid
- **WHEN** klien meminta presigned URL upload dengan kategori yang dikenal, MIME yang diizinkan, dan ukuran dalam batas
- **THEN** sistem mengembalikan `UploadPermission` berisi `presigned_url` (berlaku 10 menit) dan `object_key` yang dibuat server

#### Scenario: Kategori tidak dikenal
- **WHEN** klien meminta presigned URL upload dengan kategori yang tidak terdaftar
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`

#### Scenario: MIME tidak diizinkan untuk kategori
- **WHEN** klien meminta presigned URL upload dengan MIME yang tidak termasuk whitelist kategori tersebut
- **THEN** sistem menolak dengan `StorageClientError::InvalidMime`

#### Scenario: Ukuran melebihi batas kategori
- **WHEN** klien meminta presigned URL upload dengan `size_bytes` melebihi `max_bytes` kategori tersebut
- **THEN** sistem menolak dengan `StorageClientError::FileTooLarge`

#### Scenario: Storage tidak tersedia
- **WHEN** MinIO endpoint tidak dikonfigurasi (env vars tidak diset)
- **THEN** sistem menolak dengan `StorageClientError::Unavailable`

### Requirement: Object key deterministik dan mencegah tabrakan

Sistem SHALL membuat object key menggunakan UUID v7 (time-sorted) untuk menjamin keunikan. Sistem SHALL menetapkan ekstensi berkas berdasarkan MIME yang diklaim (`image/jpeg` → `jpg`, `image/png` → `png`, `application/pdf` → `pdf`, default → `jpg`). Sistem SHALL TIDAK menggunakan nama berkas asli dari klien.

#### Scenario: Object key unik untuk setiap permintaan
- **WHEN** klien yang sama meminta upload dua kali untuk kategori yang sama
- **THEN** setiap permintaan menghasilkan `object_key` yang berbeda (UUID v7 unik)

#### Scenario: Ekstensi ditentukan dari MIME
- **WHEN** klien meminta upload dengan MIME `image/png`
- **THEN** object key berakhiran `.png`
