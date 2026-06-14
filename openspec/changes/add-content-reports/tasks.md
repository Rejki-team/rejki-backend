## 1. Scaffold Crate & Workspace

- [x] 1.1 Buat crate `rust-services/report-service/` (struktur 4 layer: domain, application, infrastructure, interface) mengikuti pola service lain
- [x] 1.2 Buat crate `rust-services/report-service-client/` (trait + tipe publik)
- [x] 1.3 Daftarkan kedua crate sebagai member di workspace `rust-services/Cargo.toml`
- [x] 1.4 Tambah keduanya ke daftar COPY/dummy di `rust-services/Dockerfile`

## 2. Migrasi Basis Data (schema `report`)

- [x] 2.1 Buat schema `report` dan tabel `report (id UUID PK, reporter_id UUID NOT NULL, target_type TEXT NOT NULL CHECK (iklan|user), target_id UUID NOT NULL, keterangan TEXT NOT NULL, evidence_object_key TEXT, status TEXT NOT NULL DEFAULT 'pending' CHECK (pending|in_review|rejected|resolved), action_note TEXT, reviewed_by UUID, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)` + index `status`, `reporter_id`, `target_id` (up + down)
- [x] 2.2 Verifikasi migrasi naik & turun bersih pada DB test

## 3. Kategori StorageClient Baru

- [x] 3.1 Tambah kategori `report-evidence` (≤5MB, JPEG/PNG/PDF) di `StorageClient`
- [x] 3.2 Unit test: penolakan mime/ukuran tidak valid

## 4. Domain & Repository (report-service)

- [x] 4.1 Definisikan entity domain `Report` + `ReportStatus` (pending|in_review|rejected|resolved) + `ReportTargetType` (iklan|user)
- [x] 4.2 Implementasi repository: insert report, list (search/filter/sort/pagination), get_by_id, update status & action_note
- [x] 4.3 Unit test: operasi CRUD dasar

## 5. Pembuatan Aduan dari Sisi Mobile (spec: content-reporting)

- [x] 5.1 DTO `CreateReportInput { target_type, target_id, keterangan, mime, size_bytes }`
- [x] 5.2 Endpoint `POST /api/v1/reports` (auth user): minta presigned upload bukti → commit → insert report `pending`
- [x] 5.3 Integration test: buat aduan valid → 201 + evidence tersimpan; tanpa bukti → masih ok (opsional); mime/size invalid → 422

## 6. Listing & Detail Admin (spec: report-admin-workflow)

- [x] 6.1 Endpoint `GET /api/v1/admin/reports` (proteksi `require_admin`) — search by ID pengguna/ID aduan, filter/sort by status, pagination
- [x] 6.2 Endpoint `GET /admin/reports/{id}` — detail + presigned read bukti foto
- [x] 6.3 Integration test: search/filter berfungsi; non-admin → 403

## 7. Tindak Lanjut Admin (spec: report-admin-workflow)

- [x] 7.1 Endpoint `POST /api/v1/admin/reports/{id}/review` {approved:bool, action_note}: approve → resolved, reject → rejected; **action_note wajib**
- [x] 7.2 Tombol Tolak/Terima terkunci setelah status bukan `pending`/`in_review`
- [x] 7.3 Notifikasi email + in-app ke pelapor (dan pihak diadukan bila relevan)
- [x] 7.4 Integration test: approve/reject + notifikasi; tanpa action_note → 422; verifikasi ulang → 409

## 8. Export CSV

- [x] 8.1 Endpoint `GET /api/v1/admin/reports/export.csv` (proteksi admin) mengikuti filter aktif
- [x] 8.2 Integration test: CSV berisi header + baris sesuai filter

## 9. ReportClient (spec: content-reporting)

- [x] 9.1 Definisikan trait `ReportClient` di `report-service-client`: `create_report`, `get_report_status`
- [x] 9.2 Implementasi `ReportInProcessClient`

## 10. Wiring & Finalisasi

- [x] 10.1 Wire router report-service & `ReportInProcessClient` di `rejki-app` di bawah prefix `/api/v1/reports` & `/api/v1/admin/reports`
- [x] 10.2 Wire `require_admin`, `StorageClient`, `NotificationClient`
- [x] 10.3 Pastikan envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`
- [x] 10.4 `cargo fmt`, `clippy -D warnings`, seluruh test hijau
