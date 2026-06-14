## 1. Migrasi basis data (dokumentasi, tidak dieksekusi)

- [ ] 1.1 Tulis migrasi: `DROP COLUMN harga, kondisi`
- [ ] 1.2 Tulis migrasi: `ADD COLUMN jenis_barang TEXT NOT NULL CHECK (jenis_barang IN ('bekas','baru'))`, `jumlah INT NOT NULL CHECK (jumlah >= 1)`, `lokasi_pengambilan TEXT`
- [ ] 1.3 Tulis migrasi: ganti `is_sold BOOLEAN` → `availability_status TEXT NOT NULL DEFAULT 'tersedia' CHECK (availability_status IN ('tersedia','sudah_diambil'))`
- [ ] 1.4 Dokumentasikan strategi backfill data existing (default jenis_barang='bekas', jumlah=1, lokasi_pengambilan←lokasi)

## 2. Domain & DTO (spec: barang-bekas-gratis-model)

- [ ] 2.1 `entity.rs`: hapus `harga`, `kondisi`, `is_sold`; tambah `jenis_barang`, `jumlah`, `lokasi_pengambilan`, `availability_status`
- [ ] 2.2 `dto.rs`: `CreateIklanBarangBekasInput` / `UpdateIklanBarangBekasInput` — hapus harga/kondisi, tambah field baru + validasi (jumlah ≥ 1, jenis_barang enum)
- [ ] 2.3 `dto.rs`: `IklanBarangBekasResponse` & `AdminIklanBarangBekasResponse` — surface kolom baru + availability_status

## 3. Service & repository

- [ ] 3.1 `pg_repository.rs`: INSERT/SELECT kolom baru (hapus harga/kondisi)
- [ ] 3.2 `service.rs`: mapping create/update; ganti `mark_sold` → `mark_taken` (set availability_status='sudah_diambil')
- [ ] 3.3 `interface/mod.rs`: route `PATCH /{id}/sold` → `PATCH /{id}/taken`
- [ ] 3.4 `interface/handlers.rs`: handler `mark_taken`; CSV header + baris memuat jenis_barang, jumlah, lokasi_pengambilan, availability_status

## 4. Lintas-aplikasi & dokumentasi

- [ ] 4.1 Catat prasyarat penyesuaian form create di Rejki Mobile (prd-mobile)
- [ ] 4.2 Update OpenAPI bila barang bekas terdokumentasi di `rejki-app/openapi.rs`

## 5. Pengujian

- [ ] 5.1 Test create dengan field baru; tolak jumlah < 1; tolak jenis_barang invalid
- [ ] 5.2 Test mark_taken mengubah availability_status; idempoten
- [ ] 5.3 Test listing admin & CSV memuat kolom baru
- [ ] 5.4 Test status moderasi (suspend) terpisah dari availability

## 6. Verifikasi

- [ ] 6.1 `cargo build` workspace hijau (online + offline `.sqlx`)
- [ ] 6.2 `cargo fmt` + `cargo clippy` bersih
- [ ] 6.3 Cek silang: requirement spec ↔ test ↔ FR-ADM-GDS-01/02 di PRD
