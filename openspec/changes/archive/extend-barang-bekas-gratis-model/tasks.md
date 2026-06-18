## 1. Migrasi basis data (dokumentasi, tidak dieksekusi)

- [x] 1.1 Tulis migrasi: `DROP COLUMN harga, kondisi`
- [x] 1.2 Tulis migrasi: `ADD COLUMN jenis_barang TEXT NOT NULL CHECK (jenis_barang IN ('bekas','baru'))`, `jumlah INT NOT NULL CHECK (jumlah >= 1)`, `lokasi_pengambilan TEXT`
- [x] 1.3 Tulis migrasi: ganti `is_sold BOOLEAN` → `availability_status TEXT NOT NULL DEFAULT 'tersedia' CHECK (availability_status IN ('tersedia','sudah_diambil'))`
- [x] 1.4 Dokumentasikan strategi backfill data existing (default jenis_barang='bekas', jumlah=1, lokasi_pengambilan←lokasi) — **lihat design.md D5**

## 2. Domain & DTO (spec: barang-bekas-gratis-model)

- [x] 2.1 `entity.rs`: hapus `harga`, `kondisi`, `is_sold`; tambah `jenis_barang`, `jumlah`, `lokasi_pengambilan`, `availability_status`
- [x] 2.2 `dto.rs`: `CreateIklanBarangBekasInput` — hapus harga/kondisi, tambah field baru + validasi (jumlah ≥ 1, jenis_barang enum via custom validator)
- [x] 2.3 `dto.rs`: `IklanBarangBekasResponse` & `AdminIklanBarangBekasResponse` — surface kolom baru + availability_status

## 3. Service & repository

- [x] 3.1 `pg_repository.rs`: INSERT/SELECT kolom baru (hapus harga/kondisi) — 17 konstanta SQL literal statis
- [x] 3.2 `service.rs`: mapping create; ganti `mark_sold` → `mark_taken` (set availability_status='sudah_diambil', atomik WHERE tersedia)
- [x] 3.3 `interface/mod.rs`: route `PATCH /{id}/sold` → `PATCH /{id}/taken`
- [x] 3.4 `interface/handlers.rs`: handler `mark_taken`; CSV header + baris memuat jenis_barang, jumlah, lokasi_pengambilan, availability_status

## 4. Lintas-aplikasi & dokumentasi

- [x] 4.1 Catat prasyarat penyesuaian form create di Rejki Mobile (prd-mobile)
- [x] 4.2 Barang bekas belum terdokumentasi di `rejki-app/openapi.rs` (tidak ada anotasi utoipa existing) — **no-op**

## 5. Pengujian

- [x] 5.1 Test create dengan field baru; tolak jumlah < 1; tolak jenis_barang invalid
- [x] 5.2 Test mark_taken mengubah availability_status; idempoten (atomik)
- [x] 5.3 Test listing publik hanya menampilkan "tersedia" (filter availability)
- [x] 5.4 Test response GET memuat kolom gratis, tanpa field jual-beli

## 6. Verifikasi

- [x] 6.1 `cargo build` workspace hijau (online + offline `.sqlx`)
- [x] 6.2 `cargo fmt` + `cargo clippy` bersih
- [x] 6.3 Cek silang: requirement spec ↔ test ↔ FR-ADM-GDS-01/02 di PRD
- [x] 6.4 Regression: 13/13 auth_integration_test + 13/13 user_admin_kyc_test lulus
