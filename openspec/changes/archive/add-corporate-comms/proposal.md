## Why

Rejki Web Dashboard ([prd-dashboard.md](../../../docs/prd/prd-dashboard.md) §5.9) menuntut halaman **Corporate Communication** untuk admin menerbitkan artikel/blog, menyunting, menghapus, dan **menyiarkan notifikasi in-app ke seluruh pengguna** saat artikel terbit atau diperbarui. Backend saat ini **tidak memiliki domain artikel** — tidak ada tabel artikel, endpoint, maupun mekanisme broadcast ke seluruh pengguna (walaupun `NotificationClient.send_bulk` sudah ada). Proposal ini menyediakan domain artikel korporat + broadcast, diproteksi RBAC (`add-admin-rbac`).

## What Changes

- **Domain artikel/blog** (FR-ADM-COM-01..04): tabel `corporate_article` (id, author_id, category DEFAULT 'informasi', title, body, photo_object_key, created_at, updated_at). CRUD admin: list (search judul, sort kategori), create, edit, delete (soft). Pembuat artikel read-only (dari sesi admin). Kategori awal hanya `informasi` (dropdown), dirancang extensible (CHECK atau enum terpisah).
- **Broadcast notifikasi in-app ke seluruh pengguna** (FR-ADM-COM-05): saat artikel baru terbit atau diperbarui, sistem menyiarkan notifikasi in-app ke **seluruh** pengguna via `NotificationClient.send_bulk`. Tidak ada segmentasi pada tahap ini.
- **Foto artikel** via `StorageClient` (kategori baru `article-photo`, ≤5MB, JPEG/PNG).
- **Listing admin** (FR-ADM-COM-01/06): `GET /api/v1/admin/articles` dengan search (judul), sort kategori, pagination. Tidak ada export CSV untuk comms (tidak diminta User Story, hanya search & sort).

## Capabilities

### New Capabilities
- `corporate-comms-articles`: CRUD artikel (list, create, edit, delete) dengan foto dan kategori.
- `comms-broadcast`: broadcast notifikasi in-app ke seluruh pengguna saat artikel terbit atau diperbarui.

## Impact

- **Kode**: crate baru `rust-services/corporate-comms-service/` (domain, application, infrastructure, interface) dan `rust-services/corporate-comms-service-client/` (trait + tipe), menambah 1 pasang member workspace; wiring di `rejki-app`.
- **Basis data** (schema `comms`): tabel `corporate_article (id UUID PK, author_id UUID NOT NULL, category TEXT NOT NULL DEFAULT 'informasi' CHECK (informasi), title TEXT NOT NULL, body TEXT NOT NULL, photo_object_key TEXT, deleted_at TIMESTAMPTZ, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)` + index `category`. Tanpa FK lintas-schema.
- **API**: endpoint admin `GET /admin/articles` (search/sort), `POST /admin/articles` (create), `GET /admin/articles/{id}`, `PUT /admin/articles/{id}` (edit), `DELETE /admin/articles/{id}` (soft). Seluruh `/admin/**` diproteksi `require_admin`. Tidak ada endpoint publik (artikel dikonsumsi via notifikasi in-app; bila perlu endpoint baca publik di luar scope).
- **Dependensi**: `add-admin-rbac` (proteksi), `StorageClient` (kategori `article-photo`), `NotificationClient.send_bulk()` (broadcast).
- **Standar**: envelope `ApiResponse`, error RFC 9457-inspired, soft-delete, propagasi `request_id`, snake_case DB.

## Non-Goals

- Segmentasi audiens broadcast (semua pengguna, tidak ada filter).
- Penjadwalan publikasi (semua artikel langsung terbit).
- Riwayat broadcast atau analytics keterbacaan.
- Endpoint publik baca artikel (bila kelak diperlukan, di luar scope).
- Rich text/WYSIWYG editor di backend (body = plain text/markdown; editor di sisi Vue).
- UI dashboard — `add-rejki-web-dashboard`.
