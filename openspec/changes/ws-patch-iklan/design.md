## Context

4 iklan service saat ini tidak punya endpoint update. Pattern yang sudah ada: `AppState` struct dengan `svc: Arc<Service<R>>`, handler di `interface/handlers.rs`, route di `interface/mod.rs`. Semua service menggunakan `anyhow::Error` (bukan `ServiceError` enum). Ownership check di SQL (`WHERE id=$1 AND owner_id=$2`).

## Decisions

### D1: Partial update (PATCH) — semua field `Option<T>`, bukan PUT full replacement

**Pilihan**: DTO dengan semua field opsional. COALESCE di SQL: `field = COALESCE($N, field)`.

**Dibanding PUT**: PUT mengharuskan client mengirim semua field — lebih berat untuk mobile/web client. PATCH lebih idiomatis.

### D2: Guard lifecycle di service layer, bukan di SQL

**Pilihan**: Service method `update()` melakukan `find_by_id()` dulu, cek `moderation_status` + `availability_status` (barang) / `status` (pelatihan), lalu lanjut ke `repo.update()`.

**Dibanding guard di SQL**: Filter di SQL (`WHERE moderation_status='active'`) lebih sederhana tapi tidak bisa bedakan "tidak ditemukan" vs "di-suspend". Dengan cek di service, kita bisa return error spesifik untuk suspended.

### D3: Response 200 dengan entity yang sudah di-update

**Pilihan**: PATCH return 200 OK + `ApiResponse<T>` berisi entity setelah update. Sesuai API standard docs.

### D4: Rate limit 30 req/menit per user

**Pilihan**: Pakai `self.rate_limiter.check_rate_limit(...)` yang sudah ada dari W3A-02. Konsisten dengan endpoint create.

## Risks / Trade-offs

- **Risk**: COALESCE SQL pattern tidak handle `null` vs `not provided` — untuk field `Option<T>` di DB (lokasi, region_id), client tidak bisa set ke `None` via PATCH (karena COALESCE skip jika None) → Acceptable untuk v1. Jika perlu nullify, pakai sentinel value di iterasi berikutnya.
- **Risk**: find_by_id + update = 2 query → race condition (update by admin between check and update) → Acceptable untuk sekarang (low probability, non-critical data).
