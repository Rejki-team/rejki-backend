## Context

Storage service adalah service infrastruktur shared dalam modular monolith Rejki. Ia menyediakan upload/download objek ke MinIO (S3-compatible object storage) via presigned URL. Service ini sudah berjalan penuh — semua domain lain (auth, user, iklan-*, corporate-comms) menggunakannya melalui `StorageClient` trait secara in-process. Service ini dibangun sebagai bagian Phase 1.5 foundation (W0-W9) sebelum workflow OpenSpec diterapkan.

**Arsitektur saat ini:**
- `storage-service-client/` — trait `StorageClient` + tipe publik (`FileInfo`, `UploadPermission`, `StorageClientError`) + fungsi validasi (`validate_file`, `verify_magic_bytes`)
- `storage-service/` — implementasi in-process: `StorageInProcessClient` (implementasi trait) → `MinioStorage` (low-level MinIO SDK wrapper via `aws-sdk-s3`)
- Tidak memiliki `domain/` atau `application/` layer — ini adalah service infrastruktur murni, bukan domain service

**Pola penggunaan:**
1. Klien (frontend/mobile) meminta presigned URL ke backend via endpoint domain (mis. `POST /users/me/avatar`)
2. Domain service memanggil `StorageClient::request_upload(category, user_id, file_info)`
3. `StorageInProcessClient` memvalidasi MIME + ukuran per kategori, membuat `object_key` server-side (`uploads/{category}/{user_id}/{uuid}.{ext}`), dan menghasilkan presigned URL ke MinIO
4. Klien mengunggah langsung ke MinIO via presigned URL (backend tidak menyentuh byte berkas)
5. Commit: klien mengirim `object_key` + `magic_head_b64` ke backend → backend verifikasi magic bytes → simpan referensi

## Goals / Non-Goals

**Goals:**
- Mendokumentasikan kontrak `StorageClient` trait secara formal
- Mendokumentasikan 8 kategori storage yang sudah terdaftar beserta aturan validasinya
- Mendokumentasikan mekanisme keamanan (magic bytes, object key server-side, presigned TTL)
- Menetapkan prosedur standar penambahan kategori baru untuk change mendatang

**Non-Goals:**
- Public URL (semua akses via presigned URL berumur pendek)
- Versioning objek
- Lifecycle policies objek
- Multi-region replication
- CDN / edge caching
- Enkripsi server-side (objek sudah dienkripsi di level aplikasi sebelum upload — mis. NIK)
- Pembatasan akses baca per pengguna di level storage (dilakukan di level aplikasi via ownership query)

## Decisions

### D1 — Presigned URL, bukan proxy backend
Backend TIDAK mem-proxy byte berkas. Klien mengunggah/mengunduh langsung ke/dari MinIO via presigned URL. Ini menghindari bottleneck bandwidth backend dan memanfaatkan MinIO sebagai object storage dedicated. Konsekuensi: backend tidak bisa memindai konten berkas (hanya verifikasi magic bytes dari header yang dikirim klien saat commit).

### D2 — Object key dibuat server-side, bukan client-provided
Format: `uploads/{category}/{user_id}/{uuid}.{ext}`. Server membuat UUID v7 untuk mencegah collision dan nama deterministik. Ekstensi ditentukan dari MIME yang diklaim (bukan dari nama file asli). Object key TIDAK bisa dipilih klien — mencegah path traversal dan overwrite.

### D3 — Kategori storage sebagai match statement, bukan tabel DB
Kategori disimpan sebagai `match category { ... }` di `StorageInProcessClient::request_upload()`. Ini sengaja — kategori adalah aturan bisnis yang berubah jarang dan harus direview saat ditambah (setiap penambahan = perubahan kode yang di-review). Alternatif "tabel DB" ditolak karena over-engineering untuk 8 kategori; alternatif "konfigurasi file" ditolak karena validasi butuh kustomisasi (ekstensi mapping, prefix path).

### D4 — Dua langkah: request → upload → commit
Alur upload adalah tiga langkah untuk keamanan:
1. **Request**: klien kirim MIME + size_bytes → server validasi → kembalikan presigned URL + object_key
2. **Upload**: klien unggah langsung ke MinIO (backend tidak terlibat)
3. **Commit**: klien kirim object_key + magic_head_b64 → server verifikasi magic bytes → simpan referensi permanen

Ini mencegah klien mengklaim MIME palsu (mis. skrip `.exe` diganti nama jadi `.jpg`) karena magic bytes diverifikasi dari byte awal berkas asli yang sudah terunggah.

### D5 — Presigned TTL: upload 10 menit, download 5 menit
Upload TTL 600 detik — cukup untuk klien mobile di jaringan lambat. Download TTL 300 detik — cukup untuk melihat dokumen, tapi tidak bisa dibagikan sebagai URL permanen.

### D6 — Kategori tidak memiliki concept "pemilik" di storage layer
Meskipun object key mengandung `user_id`, otorisasi akses dilakukan di application layer (domain service). Storage service tidak tahu konsep kepemilikan — ia hanya memvalidasi MIME/ukuran dan menghasilkan presigned URL. Ini menjaga storage service tetap sederhana (infrastruktur, bukan domain).

### D7 — MinioStorage: graceful degradation
`MinioStorage::from_env()` mengembalikan `Option<Self>` — `None` bila env vars tidak diset. `StorageInProcessClient` menyimpan `Option<MinioStorage>` dan mengembalikan `StorageClientError::Unavailable` bila storage tidak tersedia. Ini memungkinkan development tanpa MinIO (presigned URL tidak bisa dibuat, tapi aplikasi tetap berjalan).

## Risks / Trade-offs

- **MinIO sebagai single point of failure** → mitigasi: MinIO mendukung replikasi; untuk production, deploy MinIO cluster dengan erasure coding. Storage service sendiri stateless — bisa di-restart tanpa data loss.
- **Presigned URL bisa disalahgunakan bila bocor** → mitigasi: TTL pendek (5-10 menit), URL hanya diberikan ke klien terautentikasi, object key tidak bisa ditebak (UUID v7).
- **Magic bytes verification tidak 100% akurat** → mitigasi: header byte adalah pertahanan pertama; konten berkas tetap disimpan di MinIO yang tidak bisa dieksekusi server-side. Untuk keamanan tambahan, bisa ditambah ClamAV scanning di masa depan (di luar scope).
- **Kategori bertambah tanpa standarisasi** → mitigasi: penambahan kategori baru harus melalui change OpenSpec yang merujuk spec ini, sehingga ada review formal.

## Migration Plan

Dokumen ini tidak memerlukan migrasi — kode sudah berjalan. Spec ini hanya mendokumentasikan state saat ini.
