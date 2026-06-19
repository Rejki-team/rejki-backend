## Mengapa

4 iklan service (barang-bekas, pekerja, pekerjaan, pelatihan) hanya punya endpoint `POST /` (create) dan `DELETE /{id}` (delete). User tidak bisa mengupdate iklan setelah dibuat — harus hapus lalu buat ulang. Ini merusak UX: data seperti deskripsi, harga, lokasi sering perlu diperbaiki tanpa harus membuat iklan baru.

## What Changes

- **4 service dapat endpoint baru**: `PATCH /api/v1/{resource}/{id}` — partial update (hanya field yang dikirim berubah)
- **DTO baru**: `Update*Input` — semua field `Option<T>`, reusable untuk PATCH
- **Repository method baru**: `update(id, owner_id, params) → Result<Option<Entity>>` — COALESCE SQL pattern
- **Service method baru**: `update()` — rate limit + cooldown + sanitasi + lifecycle guard + region validation
- **Handler baru**: mapping error ke `AppError` (NotFound untuk IDOR, Forbidden untuk lifecycle violation)
- **Route protected**: `PATCH /{id}` di bawah middleware require_auth + require_active_account

## New Capabilities

- `patch-iklan`: Partial update endpoint untuk 4 iklan service dengan ownership check di SQL (404 untuk IDOR), lifecycle guard, rate limit

## Impact

- **4 service interface**: route baru + handler baru
- **4 service application**: DTO baru + service method baru
- **4 service domain**: repository trait method baru
- **4 service infrastructure**: SQL UPDATE query baru
- **Swagger**: update openapi.rs
