## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.7, FR-ADM-USR-*) menuntut halaman **Pengelolaan Pengguna** tempat admin meninjau **daftar pengajuan verifikasi data diri (KYC)**, membuka detail (NIK ter-mask + **click-to-view** Foto KTP/Selfie yang **teraudit**), lalu menyetujui/menolak. Penelusuran kode aktual ([dashboard-gap-analysis.md](../../../docs/dashboard-gap-analysis.md) §2.5) menemukan backend **belum** menyediakan tiga hal yang dibutuhkan halaman ini:

1. **Listing pengajuan KYC untuk admin tidak ada** — `user-service` admin hanya punya `POST /admin/kyc/{id}/review` ([user-service/interface/mod.rs:74-84](../../../rust-services/user-service/src/interface/mod.rs#L74)). Tanpa endpoint daftar, seluruh halaman tidak dapat dirender.
2. **Admin tidak dapat membaca dokumen KTP/Selfie pengguna lain** — `GET /me/documents/{kind}` bersifat **self-only** ([mod.rs:64](../../../rust-services/user-service/src/interface/mod.rs#L64)). Kebutuhan ini sudah dicatat tertulis sebagai pending di `add-user-service-kyc/tasks.md` baris 37 (`[6.3 admin] Baca dokumen oleh admin: menunggu RBAC`). RBAC kini tersedia (`add-admin-rbac` selesai), sehingga blocker dapat dibuka.
3. **Auto-hapus dokumen saat penolakan KYC tidak ter-wire** — `review_kyc(approved=false)` tidak memanggil `purge_documents` ([service.rs:222-296](../../../rust-services/user-service/src/application/service.rs#L222)); method `purge_documents` ada tetapi "tanpa HTTP trigger auto" ([service.rs:427-428](../../../rust-services/user-service/src/application/service.rs#L427)). User Story mewajibkan dokumen **terhapus otomatis setelah penolakan** (UU PDP).

## What Changes

- **Listing pengajuan KYC admin** (FR-ADM-USR-01/08): endpoint `GET /api/v1/users/admin/kyc` (proteksi `require_admin`), kolom sesuai User Story (ID, Nama, Tingkat Pendidikan, Jenis Kelamin, TTL, Alamat Domisili, kombinasi wilayah Kelurahan→Negara, Status Verifikasi). Mendukung search (`q` by ID/Nama), filter/sort by status, pagination `limit`/`offset` dengan envelope `ApiResponse` + `PaginatedMeta`. NIK **tidak** disertakan di listing; hanya `nik_masked` bila perlu.
- **Detail pengajuan KYC admin** (FR-ADM-USR-02): `GET /api/v1/users/admin/kyc/{id}` menampilkan data diri lengkap (NIK tetap ter-mask) untuk popup detail.
- **Akses dokumen admin teraudit (click-to-view)** (FR-ADM-USR-02): `GET /api/v1/users/admin/kyc/{id}/documents/{kind}` (`kind` = `ktp|selfie`) di belakang `require_admin`, mengembalikan presigned read URL **dan mencatat akses** ke `document_access_log` (action `read_issued`, aktor = admin). Menutup `[6.3 admin]`.
- **Auto-purge dokumen saat penolakan** (FR-ADM-USR-05): `review_kyc(approved=false)` memicu `purge_documents` (hapus KTP/Selfie dari storage + kosongkan referensi). Menutup jalur penolakan dari `[6.4 trigger]`.
- **Export CSV** (FR-ADM-USR-08): `GET /api/v1/users/admin/kyc/export.csv` mengikuti filter/search aktif.
- **Penguncian pasca-verifikasi** (FR-ADM-USR-06): submission yang sudah `approved|rejected` tidak dapat direview ulang (idempotensi + integritas audit).

## Capabilities

### New Capabilities
- `user-admin-listing`: daftar + detail pengajuan KYC untuk admin (search/filter/sort/pagination/CSV), kolom sesuai User Story, NIK ter-mask.
- `admin-document-access`: akses dokumen KTP/Selfie pengguna oleh admin via presigned URL dengan pencatatan audit setiap pembukaan.
- `kyc-document-retention`: pemusnahan otomatis dokumen KYC saat pengajuan ditolak.

## Impact

- **Kode**: `rust-services/user-service/` — tambah handler admin (`admin_list_kyc`, `admin_get_kyc`, `admin_get_document`, `admin_export_csv`), method service (`admin_list_submissions`, `admin_get_document_url`), query repo listing + join wilayah; wire purge ke `review_kyc`. Tanpa crate baru.
- **Basis data**: reuse tabel `user_svc.profiles`, `user_svc.kyc_submission`, `user_svc.document_access_log` (sudah ada). Mungkin menambah index pendukung listing (`kyc_submission.status`, `created_at`). Tanpa schema baru.
- **API**: `GET /api/v1/users/admin/kyc`, `GET /api/v1/users/admin/kyc/{id}`, `GET /api/v1/users/admin/kyc/{id}/documents/{kind}`, `GET /api/v1/users/admin/kyc/export.csv` — seluruhnya `require_admin`.
- **Dependensi**: `add-admin-rbac` (proteksi `require_admin`), `StorageClient` (presigned read + delete), `RegionClient` (resolve nama wilayah untuk kolom Alamat/Wilayah), `add-user-service-kyc` (domain KYC + `document_access_log`).
- **Standar**: envelope `ApiResponse`, IDOR→404, propagasi `request_id`, snake_case DB, default-deny RBAC.

## Non-Goals

- UI dashboard (Vue) — `add-rejki-web-dashboard`.
- Bulk suspend pengguna + purge saat suspend permanen — `extend-user-suspension-bulk-purge` (Change B).
- Trigger purge saat penutupan akun (event auth-service) — di luar scope; tetap pending `[6.4 trigger]` untuk jalur non-reject.
- Dekripsi/penyajian NIK penuh di listing — NIK penuh hanya via click-to-view teraudit pada detail (bukan listing massal).
