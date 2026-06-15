## Context

Domain artikel adalah domain mandiri baru, skala kecil (satu tabel inti), mengikuti pola `region-service` dan `report-service`. `NotificationClient.send_bulk(recipient_ids)` sudah ada tetapi belum pernah dipakai untuk broadcast ke seluruh pengguna. Tantangan: mengambil daftar seluruh `user_id` untuk broadcast. `StorageClient` mendukung presigned upload. `add-admin-rbac` menyediakan `require_admin`. Acuan: FR-ADM-COM-* di PRD Dashboard.

## Goals / Non-Goals

**Goals:** CRUD artikel admin + foto + broadcast notifikasi in-app ke seluruh pengguna saat terbit/update. Kepatuhan standar Phase 1.5.

**Non-Goals:** Segmentasi audiens, penjadwalan, rich text backend, endpoint publik baca artikel.

## Decisions

### D1 — Crate baru `corporate-comms-service` + `client`
Domain mandiri. Skala kecil tetapi batas jelas — tidak menumpang di service lain. `corporate-comms-service-client` mengekspos trait `CommsClient` bila domain lain perlu membaca artikel (mis. mobile app menampilkan daftar pengumuman — di luar scope).

### D2 — Satu tabel `corporate_article` dengan soft-delete
`category TEXT NOT NULL DEFAULT 'informasi' CHECK (informasi)` — satu nilai untuk saat ini, CHECK extensible. Soft-delete dengan `deleted_at` (konsisten dengan pola iklan). `author_id` dari sesi admin (read-only). `photo_object_key` optional via `StorageClient` (kategori `article-photo`, ≤5MB, JPEG/PNG).

### D3 — Broadcast via `send_bulk` dengan daftar seluruh user_id via `AuthClient`
`NotificationClient.send_bulk(recipient_ids: Vec<Uuid>, payload)` sudah ada. Untuk broadcast ke seluruh pengguna, corporate-comms memanggil `AuthClient.list_active_user_ids()` — method baru di trait `AuthClient` yang diimplementasikan di `AuthInProcessClient` → `PgAuthRepository.list_active_user_ids()` (query `SELECT id FROM auth.users WHERE status = 'active'`). Arah panggil: **corporate-comms → auth-service-client trait** (bukan query lintas-schema langsung). Daftar user_id dapat besar; untuk basis kecil-moderat, satu panggilan trait + `send_bulk` memadai. Untuk skala besar, `list_active_user_ids` dapat di-paginasi di masa depan tanpa mengubah konsumen. Notifikasi hanya in-app (tidak email massal — terlalu mahal & risiko spam).

### D4 — Broadcast dipicu saat create & update, bukan delete
Artikel baru → broadcast "Artikel baru: {title}". Artikel diperbarui → broadcast "Artikel diperbarui: {title}". Artikel dihapus → tidak broadcast. Broadcast bersifat fire-and-forget (error di-log, tidak menggagalkan create/update).

### D5 — StorageClient kategori baru
`article-photo` (≤5MB, JPEG/PNG) — foto artikel.

### D6 — Kepatuhan standar Phase 1.5
Envelope, error RFC 9457-inspired, propagasi `request_id`, snake_case DB, soft-delete.

## Risks / Trade-offs

- **Broadcast ke seluruh pengguna** — `send_bulk` dengan ribuan ID dapat lambat; mitigasi: async/fire-and-forget, atau paginasi. Awasi performa Redis Stream.
- **Tidak ada endpoint baca publik** — bila mobile app kelak perlu menampilkan artikel, endpoint publik `GET /articles` dapat ditambah (di luar scope).
- **Kategori tunggal** — mudah diperluas; hanya ubah CHECK constraint.

## Migration Plan

1. Buat crate `corporate-comms-service` + `corporate-comms-service-client`, daftarkan ke workspace.
2. Migrasi: schema `comms` + tabel `corporate_article` + index `category`.
3. Tambah kategori `article-photo` di `StorageClient`.
4. Implementasi: CRUD admin (list/search/sort, create, edit, soft-delete) + broadcast.
5. Wire di `rejki-app` dengan `require_admin`, `StorageClient`, `NotificationClient`.
6. Rollback: migrasi turun drop schema `comms`.

## Open Questions

- Apakah broadcast perlu menyertakan isi artikel (body) di payload notifikasi? Default: hanya judul + "ketuk untuk baca"; isi penuh bisa terlalu panjang.
- Apakah perlu endpoint publik `GET /articles` untuk mobile? Default: tidak pada iterasi ini; evaluasi saat mobile app membutuhkan.
