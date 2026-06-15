## 1. Migrasi Basis Data (keempat schema iklan)

- [x] 1.1 Tambah `moderation_status TEXT NOT NULL DEFAULT 'active'` (CHECK `active|suspended_temp|suspended_permanent`) + `deleted_at TIMESTAMPTZ` pada keempat tabel iklan + index `moderation_status` (up + down)
- [x] 1.2 Buat tabel `iklan_suspension (id, iklan_id, is_permanent, reason, evidence_object_key, expires_at, created_by, created_at)` per schema + index `iklan_id`
- [x] 1.3 Tambah kolom foto (`foto_urls TEXT[]`) pada vertikal pekerja/pekerjaan/pelatihan (barang sudah ada) (up + down)
- [x] 1.4 Verifikasi migrasi naik & turun bersih pada DB test

## 2. Storage & Perbaikan Field

- [x] 2.1 Tambah kategori `iklan-suspension-evidence` (≤5MB, JPEG/PNG/PDF) di `StorageClient`
- [x] 2.2 Perbaiki INSERT field yang hilang (`lokasi`, `gaji_*`/`tarif_*`/`harga`, `tanggal_*`) di service create
- [x] 2.3 Surface `foto_urls` di create-DTO & response untuk barang-bekas; aktifkan foto di vertikal lain

## 3. Status Moderasi & Visibilitas (spec: iklan-moderation-status)

- [x] 3.1 Filter list publik: hanya `moderation_status='active' AND deleted_at IS NULL` (+ flag existing)
- [x] 3.2 Soft-delete admin (set `deleted_at`) terpisah dari hard-delete pemilik
- [x] 3.3 Auto-expire temporary suspension: scheduler `expire_temporary_suspensions()` mengembalikan `suspended_temp` → `active` saat `expires_at <= now()`

## 4. Listing Admin (spec: iklan-admin-listing)

- [x] 4.1 Endpoint `GET /api/v1/admin/{vertikal}` (proteksi `require_admin`) dengan kolom sesuai User Story termasuk status
- [x] 4.2 Pencarian `q` (judul/kode/pembuat) + filter/sort by `moderation_status` + pagination `limit/offset`
- [x] 4.3 Single-query optimization: `COUNT(*) OVER()` window function — mengurangi 2 query (COUNT + SELECT) menjadi 1 query
- [x] 4.4 Pastikan admin read-only (tidak ada endpoint edit data iklan)

## 5. Suspend per-Iklan (spec: iklan-suspension)

- [x] 5.1 Endpoint bukti `POST /admin/{vertikal}/suspend/evidence` (presigned upload, kategori `iklan-suspension-evidence`)
- [x] 5.2 Endpoint `POST /admin/{vertikal}/suspend` single/bulk: `{iklan_ids[], is_permanent, reason, evidence_object_key}` → set `moderation_status` + insert `iklan_suspension`
- [x] 5.3 Suspend sementara mengisi `expires_at`; permanen tidak
- [x] 5.4 Notifikasi email + in-app ke pemilik iklan terdampak via `NotificationClient`
- [x] 5.5 Tangani kegagalan parsial bulk (laporkan hasil per item)
- [x] 5.6 Cooldown 3 hari: poster dengan suspend permanen tidak dapat membuat iklan baru selama 3 hari (`is_poster_in_cooldown()` check di `create()`)

## 6. Media Popup (spec: iklan-media-visibility)

- [x] 6.1 Response menyertakan daftar foto agar dashboard menampilkan popup per foto
- [x] 6.2 Field `foto_urls` menggunakan `TEXT[]` di PostgreSQL dan `Vec<String>` di Rust

## 7. Export CSV (spec: iklan-csv-export)

- [x] 7.1 Endpoint `GET /admin/{vertikal}/export.csv` (proteksi admin) mengikuti filter/search aktif; `text/csv`
- [x] 7.2 Kolom CSV mengikuti kolom tabel User Story; tanpa data sensitif
- [x] 7.3 Batas atas CSV 10,000 baris untuk mencegah OOM

## 8. Type Safety

- [x] 8.1 `ModerationStatus` enum di Rust menggantikan raw `String` — derived `Serialize`/`Deserialize`/`Default`/`PartialEq`/`Eq`
- [x] 8.2 Konversi DB → Rust via `ModerationStatus::parse()` di `row_to_entity()`
- [x] 8.3 Semua DTO response pakai `ModerationStatus` langsung (serialisasi JSON tetap snake_case: `active`/`suspended_temp`/`suspended_permanent`)

## 9. Wiring & Finalisasi

- [x] 9.1 Wire `require_admin`, `StorageClient`, `NotificationClient` ke router admin keempat iklan di `rejki-app`
- [x] 9.2 Pastikan envelope `ApiResponse`, error RFC 9457-inspired, IDOR→404, propagasi `request_id`
- [x] 9.3 Swagger/OpenAPI docs untuk semua endpoint iklan di `rejki-app/src/openapi.rs`
- [x] 9.4 `cargo fmt`, `cargo clippy -- -D warnings`, seluruh compilation hijau — zero errors, zero warnings
