## Why

Rejki memiliki storage service (`rust-services/storage-service/` + `storage-service-client/`) yang sudah berjalan sebagai service infrastruktur shared. Service ini menyediakan upload/download objek ke MinIO via presigned URL, dipakai oleh seluruh domain lain (auth, user, iklan-*, corporate-comms). Namun storage service **tidak pernah didokumentasikan secara formal** — tidak ada proposal, design, spec arsip, maupun definisi kontrak tertulis. Ia dibangun sebagai bagian Phase 1.5 foundation (W0-W9) sebelum workflow OpenSpec diterapkan. Akibatnya, batas kontrak, kategori yang didukung, kebijakan keamanan (magic bytes), dan aturan ekstensi (bagaimana menambah kategori baru) hanya ada di kode — tidak ada sumber kebenaran tertulis yang bisa diaudit atau dijadikan acuan change lain.

## What Changes

- **Dokumentasi formal kontrak `StorageClient`**: trait publik (`request_upload`, `request_download`, `delete`) beserta tipe data (`FileInfo`, `UploadPermission`, `StorageClientError`), aturan validasi per kategori, dan mekanisme presigned URL.
- **Dokumentasi 8 kategori storage** yang sudah terdaftar: `avatar`, `ktp`, `selfie`, `suspension-evidence`, `iklan-suspension-evidence`, `training-transfer-evidence`, `training-certificate`, `article-photo` — masing-masing dengan batas ukuran, MIME yang diizinkan, dan domain pemilik.
- **Dokumentasi validasi keamanan**: `validate_file()` (MIME + ukuran), `verify_magic_bytes()` (anti-spoof JPEG/PNG/PDF), dan pembuatan object key server-side (`uploads/{category}/{user_id}/{uuid}.{ext}`).
- **Dokumentasi arsitektur**: `StorageInProcessClient` sebagai implementasi in-process trait yang me-wrap `MinioStorage` (low-level MinIO SDK wrapper via `aws-sdk-s3`), di-wire di `rejki-app` sebagai `Arc<dyn StorageClient>`.
- **Dokumentasi aturan penambahan kategori baru**: prosedur standar yang harus diikuti change lain saat menambah kategori storage.

Tidak ada perubahan kode. Ini murni dokumentasi formal dari state yang sudah berjalan.

## Capabilities

### New Capabilities
- `storage-presigned-upload`: presigned URL upload ke MinIO (S3-compatible) dengan validasi MIME/ukuran per kategori dan pembuatan object key server-side.
- `storage-presigned-download`: presigned URL download sementara (akses terbatas, TTL pendek) untuk dokumen non-publik.
- `storage-object-delete`: hapus objek dari storage — dipakai untuk pemusnahan dokumen sesuai retensi (K11 UU PDP).
- `storage-file-validation`: validasi magic bytes (JPEG `FF D8 FF`, PNG `89 50 4E 47`, PDF `%PDF-`) untuk mencegah spoof ekstensi/mime.
- `storage-category-registry`: registrasi kategori storage terpusat dengan batas ukuran + MIME whitelist per kategori.

### Modified Capabilities
<!-- Tidak ada — ini adalah dokumentasi pertama. Tidak ada spec arsip sebelumnya. -->

## Impact

- **Kode**: Tidak ada perubahan. Murni dokumentasi.
- **Dokumentasi**: Membuat `openspec/specs/storage/` sebagai sumber kebenaran tertulis untuk kontrak storage service yang akan dijadikan acuan oleh change mendatang.
- **Standar**: Menetapkan prosedur standar penambahan kategori baru — change lain yang membutuhkan kategori storage baru cukup merujuk spec ini dan menambah delta.
- **Dependensi**: Tidak ada. Storage service tidak bergantung pada service lain.
