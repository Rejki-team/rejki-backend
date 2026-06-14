## 1. Verifikasi Kode Existing

- [ ] 1.1 Verifikasi `StorageClient` trait di `storage-service-client/src/lib.rs` sesuai spec `storage-presigned-upload`, `storage-presigned-download`, `storage-object-delete`
- [ ] 1.2 Verifikasi `validate_file()` dan `verify_magic_bytes()` di `storage-service-client/src/lib.rs` sesuai spec `storage-file-validation` (6 unit test harus pass)
- [ ] 1.3 Verifikasi 8 kategori di `StorageInProcessClient::request_upload()` sesuai spec `storage-category-registry` (nama, max_bytes, allowed_mimes)
- [ ] 1.4 Verifikasi `MinioStorage` di `storage-service/src/infrastructure/minio.rs` — presigned TTL upload 600s, download 300s, env var graceful degradation
- [ ] 1.5 Verifikasi wiring di `rejki-app/src/main.rs` — `Arc<dyn StorageClient>` di-inject ke semua service consumer
- [ ] 1.6 Jalankan `cargo test -p storage-service-client` — pastikan 12/12 test pass
- [ ] 1.7 Jalankan `cargo clippy -p storage-service -p storage-service-client -- -D warnings` — pastikan clean

## 2. Sinkronisasi Dokumentasi

- [ ] 2.1 Pastikan `proposal.md` capability list konsisten dengan 5 spec file yang dibuat
- [ ] 2.2 Pastikan `design.md` mencakup semua 7 decision (D1-D7) dan risiko/mitigasi
- [ ] 2.3 Pastikan setiap spec memiliki minimal 1 requirement dan 1 scenario
- [ ] 2.4 Update `docs/prd/rejki-prd.md` §Lampiran — tambah storage service ke daftar service dengan referensi ke spec ini

## 3. Finalisasi

- [ ] 3.1 `cargo fmt`, `clippy -D warnings`, seluruh test hijau (verifikasi final — tidak ada perubahan kode)
- [ ] 3.2 Tandai semua task `[x]` dan siapkan untuk archive
