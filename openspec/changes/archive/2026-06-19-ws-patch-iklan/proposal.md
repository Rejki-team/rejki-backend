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

---

## Status

✅ **SELESAI** — 19 Juni 2026. Seluruh 7 task terimplementasi. Branch `feature/ws-patch-iklan` sudah merge ke `develop`.

### Ringkasan Eksekusi

- **4 PATCH endpoint**: `/api/v1/{pekerjaan,pekerja,barang,pelatihan}/{id}` — partial update via COALESCE SQL
- **DTO**: `Update*Input` struct dengan semua field `Option<T>`
- **Lifecycle guard per service**:
  - Barang: `moderation=Active` AND `availability=Tersedia`
  - Pelatihan: `moderation=Active` AND status in (`VerifikasiDiterima`, `PelatihanBelumDimulai`)
  - Pekerja/Pekerjaan: `moderation=Active`
- **is_active toggle**: via field `Option<bool>` di DTO
- **IDOR 404**: ownership check di SQL (`WHERE id=$1 AND owner_id=$2`)
- **Rate limit**: 30 req/min per user (via W3A-02)
- **Swagger**: 4 `Update*DocRequest` DTO + 4 PATCH path annotations di openapi.rs
