## 1. Scaffold Crate & Workspace

- [x] 1.1 Buat crate `rust-services/corporate-comms-service/` (struktur 4 layer: domain, application, infrastructure, interface) mengikuti pola service lain
- [x] 1.2 Buat crate `rust-services/corporate-comms-service-client/` (trait + tipe publik)
- [x] 1.3 Daftarkan kedua crate sebagai member di workspace `rust-services/Cargo.toml`
- [x] 1.4 Tambah keduanya ke daftar COPY/dummy di `rust-services/Dockerfile`

## 2. Migrasi Basis Data (schema `comms`)

- [x] 2.1 Buat schema `comms` dan tabel `corporate_article (id UUID PK, author_id UUID NOT NULL, category TEXT NOT NULL DEFAULT 'informasi' CHECK (informasi), title TEXT NOT NULL, body TEXT NOT NULL, photo_object_key TEXT, deleted_at TIMESTAMPTZ, created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ)` + index `category` (up + down)
- [x] 2.2 Verifikasi migrasi naik & turun bersih pada DB test

## 3. Kategori StorageClient Baru

- [x] 3.1 Tambah kategori `article-photo` (≤5MB, JPEG/PNG) di `StorageClient`
- [x] 3.2 Unit test: penolakan mime/ukuran tidak valid

## 4. Domain & Repository (corporate-comms-service)

- [x] 4.1 Definisikan entity domain `CorporateArticle` + `ArticleCategory`
- [x] 4.2 Implementasi repository: list (search by title, filter/sort by category, pagination, exclude deleted), get_by_id, insert, update, soft_delete
- [x] 4.3 Unit test: CRUD dasar + soft-delete

## 5. CRUD Artikel Admin (spec: corporate-comms-articles)

- [x] 5.1 DTO `CreateArticleInput { title, body, category? }` + `UpdateArticleInput`; `ArticleResponse { id, author_id, category, title, body, photo_object_key?, created_at, updated_at }`
- [x] 5.2 Endpoint `GET /api/v1/admin/articles` (proteksi `require_admin`) — search by title, sort by category, pagination
- [x] 5.3 Endpoint `POST /api/v1/admin/articles` (author_id dari AuthClaims, read-only); presigned upload foto artikel
- [x] 5.4 Endpoint `GET /admin/articles/{id}`
- [x] 5.5 Endpoint `PUT /admin/articles/{id}` (author_id tetap read-only)
- [x] 5.6 Endpoint `DELETE /admin/articles/{id}` (soft-delete, set `deleted_at`)
- [x] 5.7 Integration test: CRUD admin, search/sort, list mengecualikan deleted; non-admin → 403

## 6. Broadcast Notifikasi (spec: comms-broadcast)

- [x] 6.1 Implementasi `broadcast_to_all_users(title, body)`: panggil `AuthClient.list_active_user_ids()` (trait, bukan cross-schema query) → `NotificationClient.send_bulk()`
- [x] 6.2 Trigger broadcast saat artikel **create** & **update** (fire-and-forget; error di-log)
- [x] 6.3 Tidak broadcast saat delete
- [x] 6.4 Integration test: broadcast notifikasi terkirim ke seluruh pengguna aktif saat artikel dibuat/diperbarui

## 7. Wiring & Finalisasi

- [x] 7.1 Wire router corporate-comms-service di `rejki-app` di bawah prefix `/api/v1/admin/articles`
- [x] 7.2 Wire `require_admin`, `StorageClient`, `NotificationClient`
- [x] 7.3 Pastikan envelope `ApiResponse`, error RFC 9457-inspired, propagasi `request_id`, soft-delete
- [x] 7.4 `cargo fmt`, `clippy -D warnings`, seluruh test hijau
