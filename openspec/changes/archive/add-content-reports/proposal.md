## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.8) menuntut halaman **Pengelolaan Dukungan** tempat admin meninjau aduan (report) yang diajukan pengguna reguler terhadap iklan atau pengguna lain. Backend saat ini **tidak memiliki domain pelaporan** — tidak ada tabel report, endpoint aduan, maupun alur tindak lanjut. Aduan adalah prasyarat moderasi yang dibuat dari sisi **Rejki Mobile** (pengguna melaporkan konten), lalu **ditindaklanjuti admin** di Dashboard. Proposal ini menyediakan domain report/aduan, diproteksi RBAC (`add-admin-rbac`). Pembuatan aduan dari sisi mobile adalah prasyarat lintas-aplikasi (lihat [prd-mobile.md](../../../docs/prd/prd-mobile.md) §5.1).

## What Changes

- **Domain report/aduan**: tabel `report` (id, reporter_id, target_type `iklan|user`, target_id, keterangan, evidence_object_key, status `pending|in_review|rejected|resolved`, action_note, reviewed_by, timestamps). Bukti laporan (foto) via `StorageClient` kategori baru `report-evidence` (≤5MB, JPEG/PNG/PDF). Endpoint create dari sisi user (prasyarat mobile).
- **Listing admin** (FR-ADM-SUP-01/05): endpoint `GET /api/v1/admin/reports` (proteksi `require_admin`) dengan search (ID Pengguna/ID Aduan), filter/sort by status, pagination.
- **Pop-up detail + tindak lanjut** (FR-ADM-SUP-02/04): admin melihat detail aduan (termasuk foto bukti), **wajib mengisi tindakan**, approve (resolved) / reject (+ action_note). Tombol Tolak/Terima **terkunci** setelah verifikasi. Notifikasi otomatis email + in-app ke pihak terdampak.
- **Export CSV** (FR-ADM-SUP-05): `GET /admin/reports/export.csv`.
- **Admin tak dapat mengubah data aduan** (FR-ADM-SUP-05); hanya menindaklanjuti.

## Capabilities

### New Capabilities
- `content-reporting`: pembuatan aduan oleh pengguna dengan bukti foto, disimpan dengan status untuk ditinjau admin.
- `report-admin-workflow`: listing, detail, tindak lanjut (approve/reject + action_note wajib), notifikasi, CSV, dan penguncian pasca-verifikasi.

## Impact

- **Kode**: crate baru `rust-services/report-service/` (domain, application, infrastructure, interface) dan `rust-services/report-service-client/` (trait + tipe publik), menambah 1 pasang member workspace; wiring router + client di `rejki-app`. Mengikuti pola domain mandiri (seperti `region-service`).
- **Basis data** (schema `report`): tabel `report` (id, reporter_id, target_type TEXT CHECK iklan|user, target_id, keterangan, evidence_object_key, status DEFAULT pending CHECK pending|in_review|rejected|resolved, action_note, reviewed_by, created_at, updated_at) + index `status`, `reporter_id`, `target_id`. Tanpa FK lintas-schema.
- **API**: endpoint user `POST /api/v1/reports` (prasyarat mobile); endpoint admin `GET /admin/reports` + `GET /admin/reports/{id}` + `POST /admin/reports/{id}/review` + `GET /admin/reports/export.csv`. Seluruh `/admin/**` diproteksi `require_admin`.
- **Dependensi**: `add-admin-rbac` (proteksi), `StorageClient` (kategori `report-evidence`), `NotificationClient` (email + in-app).
- **Standar**: envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`, snake_case DB.

## Non-Goals

- Aduan otomatis (AI/automated moderation triage).
- Aduan percakapan/chat (FR-ADM-CHT-01/02 di v0.1 placeholder) — aduan konten iklan & pengguna saja sesuai User Story.
- Eskalasi multi-tier.
- UI dashboard (Vue) — `add-rejki-web-dashboard`.
