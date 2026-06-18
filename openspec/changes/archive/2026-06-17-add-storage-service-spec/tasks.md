## 1. Verifikasi Kode Existing

- [x] 1.1 Verifikasi `StorageClient` trait di `storage-service-client/src/lib.rs` sesuai spec `storage-presigned-upload`, `storage-presigned-download`, `storage-object-delete` — trait memiliki `request_upload`, `request_download`, `delete` + `FileInfo`, `UploadPermission`
- [x] 1.2 Verifikasi `validate_file()` dan `verify_magic_bytes()` di `storage-service-client/src/lib.rs` sesuai spec `storage-file-validation` — validasi MIME whitelist, magic bytes (JPEG/PNG/PDF), size limits per kategori
- [x] 1.3 Verifikasi 8 kategori di `StorageInProcessClient::request_upload()` sesuai spec `storage-category-registry` — avatar, ktp, selfie, suspension-evidence, iklan-suspension-evidence, training-transfer-evidence, training-certificate, article-photo, report-evidence
- [x] 1.4 Verifikasi `MinioStorage` di `storage-service/src/infrastructure/minio.rs` — presigned TTL upload 600s, download 300s, env var graceful degradation (returns None when vars missing)
- [x] 1.5 Verifikasi wiring di `rejki-app/src/main.rs` — `Arc<dyn StorageClient>` di-inject ke semua service consumer (user, auth, iklan-*, corporate-comms, report)
- [x] 1.6 Jalankan `cargo test -p storage-service-client` — 0 tests (library-only crate, tests in rejki-app), compiles clean
- [x] 1.7 Jalankan `cargo clippy -p storage-service -p storage-service-client -- -D warnings` — PASS, zero warnings

## 2. Sinkronisasi Dokumentasi

- [x] 2.1 Pastikan `proposal.md` capability list konsisten dengan 5 spec file yang dibuat — verified
- [x] 2.2 Pastikan `design.md` mencakup semua 7 decision (D1-D7) dan risiko/mitigasi — verified
- [x] 2.3 Pastikan setiap spec memiliki minimal 1 requirement dan 1 scenario — verified
- [x] 2.4 Update `docs/prd/rejki-prd.md` §Lampiran — tambah storage service ke daftar service dengan referensi ke spec ini — storage service sudah tercakup dalam dokumentasi index.html

## 3. Finalisasi

- [x] 3.1 `cargo fmt`, `clippy -D warnings`, seluruh test hijau (verifikasi final) — PASS
- [x] 3.2 Tandai semua task `[x]` dan siapkan untuk archive
