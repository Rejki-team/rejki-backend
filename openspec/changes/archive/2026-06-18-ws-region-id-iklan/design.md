## Context

Keempat tabel `iklan` (`iklan_pekerjaan.iklan`, `iklan_pekerja.iklan`, `iklan_barang_bekas.iklan`, `iklan_pelatihan.iklan`) saat ini menyimpan lokasi sebagai `lokasi TEXT` (nullable). `region-service` sudah menyediakan hirarki wilayah (provinsi→kab→kec→kel) dan client `RegionClient` dengan method `get_region(id)` untuk validasi satu kode wilayah.

User-service sudah mengintegrasikan `RegionClient` untuk validasi KYC — pola yang sama diterapkan ke iklan.

## Goals / Non-Goals

**Goals:**
- Tambah kolom `region_id TEXT` di keempat tabel iklan sebagai FK logis ke `region_service.regions(id)`
- Validasi referensial via `RegionClient::get_region(region_id)` saat create (dan update untuk pelatihan)
- Pertahankan `lokasi` free-text — tidak dihapus/diubah
- `region_id` opsional — iklan tanpa `region_id` tetap valid

**Non-Goals:**
- Tidak menambah FK constraint fisik (`REFERENCES region_service.regions`) — region data ada di service berbeda, cross-schema FK dilarang oleh §4.3 CLAUDE.md
- Tidak mengubah tampilan UI/frontend
- Tidak melakukan backfill massal `lokasi` → `region_id` (itu tanggung jawab W3C-11 nanti)
- Tidak menambah filter/list-by-region di endpoint publik (itu fitur terpisah)

## Decisions

### D1: `region_id` sebagai `TEXT`, bukan UUID

Region ID dari `region-service` adalah kode wilayah string (mis. "32.73" untuk Kota Bandung), bukan UUID. Kolom disimpan sebagai `TEXT` di keempat tabel.

### D2: Validasi via `RegionClient::get_region()`, bukan `validate_chain()`

`validate_chain()` memvalidasi rantai penuh provinsi→kab→kec→kel. Untuk iklan, cukup validasi bahwa satu `region_id` yang dimasukkan benar-benar ada di tabel region. Gunakan `get_region(region_id)` yang mengembalikan `NotFound` bila kode tidak valid.

### D3: Validasi di application layer, bukan middleware

Pattern yang sama dengan user-service: validasi region dipanggil di method `create()` / `update_pelatihan()` pada application service, bukan di middleware/extractor. Bila `RegionClient::get_region()` gagal (NotFound/Unavailable), kembalikan error yang sesuai:
- `NotFound` → 422 Validation error ("region_id tidak ditemukan")
- `Unavailable` → 503 Service Unavailable (fail-safe — jangan tolak create hanya karena region-service down)

### D4: `region_id` opsional (nullable)

Kolom `region_id` nullable. Iklan yang tidak mengisi `region_id` tetap diterima — ini backward-compatible. Hanya jika `region_id` diisi (non-empty), validasi `get_region()` dijalankan.

### D5: Injeksi `RegionClient` ke service layer, bukan handler

Service layer menerima `Option<Arc<dyn RegionClient>>` (polymorphic, sama seperti `RateLimiter`). Handler/router tidak tahu RegionClient. Wiring dilakukan di `rejki-app` (Composition Root).

### D6: Mirror W3A-02 injection pattern

Gunakan pola yang sama dengan `RateLimiter`: `with_region_client(mut self, rc: Arc<dyn RegionClient>) -> Self`. Service tetap bisa di-test tanpa RegionClient (mock/none).

## Risks / Trade-offs

- **[Kolom nullable]** → Awalnya banyak iklan tidak punya `region_id`. Backfill bisa dilakukan bertahap oleh W3C-11.
- **[No DB-level FK]** → Bisa ada `region_id` yang menjadi orphan jika region-service menghapus data. Risiko rendah — data region hampir tidak pernah dihapus.
- **[region-service Unavailable = 503]** → Bisa memblokir create iklan jika region-service down. Trade-off: validasi lebih penting daripada throughput; tapi bisa diperdebatkan untuk fail-open. Keputusan: **fail-close** untuk NotFound, **fail-open** untuk Unavailable (anggap valid, log warning).

## Migration Plan

1. Tambah migration `.up.sql` baru di keempat service: `ALTER TABLE ... ADD COLUMN region_id TEXT;` + partial index
2. Deploy kode baru (backward-compatible — kolom baru nullable, tidak ada NOT NULL)
3. Backfill bertahap oleh W3C-11 (region validation service)

**Rollback**: `ALTER TABLE ... DROP COLUMN region_id;` (migration `.down.sql`)

## Open Questions

*(tidak ada — semua keputusan sudah jelas dari W3A-02 pattern dan user-service KYC integration)*
