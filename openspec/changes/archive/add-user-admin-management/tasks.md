## 1. Listing & detail pengajuan KYC (spec: user-admin-listing)

- [x] 1.1 DTO `AdminKycListQuery { q, status, sort_by, sort_dir, limit, offset }` + `AdminKycListItem` (ID, Nama, Pendidikan, Gender, TTL, Alamat, id wilayah, Status, nik_masked)
- [x] 1.2 Repo `admin_list_submissions(query) -> (Vec<row>, total)` dengan filter/sort/pagination; index `kyc_submission(status, created_at)` bila perlu
- [x] 1.3 Service `admin_list_submissions` — map row → item; resolusi wilayah (sertakan id; UI resolve via region-service)
- [x] 1.4 Handler `admin_list_kyc` → `GET /api/v1/users/admin/kyc` (`ApiResponse::with_meta` + `PaginatedMeta`)
- [x] 1.5 DTO `AdminKycDetail` + service `admin_get_submission(id)` + handler `admin_get_kyc` → `GET /api/v1/users/admin/kyc/{id}` (NIK ter-mask; IDOR→404)
- [x] 1.6 Handler `admin_export_csv` → `GET /api/v1/users/admin/kyc/export.csv` (ikuti filter aktif; escape CSV)
- [x] 1.7 Daftarkan route di router admin user-service (`nest("/admin", ...)` + `require_admin` + `require_auth`)

## 2. Akses dokumen admin teraudit (spec: admin-document-access)

- [x] 2.1 Service `admin_get_document_url(submission_id, kind, admin_id, request_id)` — resolve submission by id (bukan claims.user_id), validasi kind ∈ {ktp, selfie}
- [x] 2.2 Catat `log_document_access(admin_id, key, ReadIssued, request_id)` (aktor = admin) sebelum menerbitkan presigned URL
- [x] 2.3 Tangani dokumen sudah dimusnahkan → kembalikan keadaan tidak-tersedia (bukan 500)
- [x] 2.4 Handler `admin_get_document` → `GET /api/v1/users/admin/kyc/{id}/documents/{kind}` (proteksi `require_admin`)

## 3. Pemusnahan & penguncian (spec: kyc-document-retention)

- [x] 3.1 Pada `review_kyc(approved=false)`: panggil `purge_documents(profile.id)` setelah status di-set Rejected & notifikasi terkirim (best-effort + logged)
- [x] 3.2 Guard idempotensi: `review_kyc` menolak bila submission sudah berstatus terminal (approved|rejected)
- [x] 3.3 Pastikan `purge_documents` aman dipanggil ulang (early-return bila tak ada dokumen — sudah ada)

## 4. OpenAPI & dokumentasi

- [x] 4.1 Tambah anotasi `utoipa::path` untuk 4 endpoint baru di `rejki-app/openapi.rs`
- [x] 4.2 Update `add-user-service-kyc/tasks.md` baris `[6.3 admin]` → resolved (tertutup oleh change ini)

## 5. Pengujian

- [x] 5.1 Test listing: pagination, search, sort, filter status; NIK tidak penuh
- [x] 5.2 Test akses dokumen: mencatat audit dengan aktor admin; kind invalid ditolak; dokumen terhapus → tidak-tersedia
- [x] 5.3 Test reject memicu purge (dokumen hilang dari storage + referensi kosong)
- [x] 5.4 Test review ulang submission terminal ditolak
- [x] 5.5 Test non-admin ditolak pada seluruh endpoint `/admin/kyc/**`

## 6. Verifikasi

- [x] 6.1 `cargo build` workspace hijau (online + offline `.sqlx`)
- [x] 6.2 `cargo fmt` + `cargo clippy` bersih
- [x] 6.3 Cek silang: setiap requirement spec punya test pendukung; keterlacakan ke FR-ADM-USR-* di PRD
