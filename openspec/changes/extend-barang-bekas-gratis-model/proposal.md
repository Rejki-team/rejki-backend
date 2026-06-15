## Why

Menu dashboard bernama **"Iklan Barang Bekas Gratis"** dan User Story (§ Iklan Barang Bekas Gratis) mendaftarkan kolom: **Jenis Barang (Bekas/Baru)**, **Foto Barang**, **Jumlah**, **Lokasi Pengambilan**, dan **Status** dengan nilai termasuk **"Sudah Diambil"**. Penelusuran kode aktual ([dashboard-gap-analysis.md](../../../docs/dashboard-gap-analysis.md) §2.4) menemukan entity backend justru bermodel **jual-beli**: `harga: i64`, `kondisi: String`, `is_sold: bool` ([iklan-barang-bekas-service/domain/entity.rs:45-49](../../../rust-services/iklan-barang-bekas-service/src/domain/entity.rs#L45)). Tidak ada `jenis_barang`, `jumlah`, maupun `lokasi_pengambilan`. Model "gratis/donasi" yang diminta User Story belum terwakili.

Pemilik produk telah memutuskan **mengubah model ke gratis/donasi**: hapus `harga`/`kondisi` (semantik jual-beli), tambah `jenis_barang` (Bekas/Baru), `jumlah`, `lokasi_pengambilan`, dan jadikan status mencerminkan **Baru → Sudah Diambil** (selain Suspended dari moderasi).

## What Changes

- **Model data gratis/donasi** (FR-ADM-GDS-01): entity & kolom DB disesuaikan — **hapus** `harga` dan `kondisi`; **tambah** `jenis_barang` (CHECK `bekas|baru`), `jumlah` (≥1), `lokasi_pengambilan` (teks). Pertahankan `foto_urls`, `moderation_status`, `is_active`, timestamps.
- **Status ketersediaan** (FR-ADM-GDS-01): semantik `is_sold` digantikan **status ketersediaan** `tersedia|sudah_diambil` (label dashboard: "Baru/Tersedia" dan "Sudah Diambil"). Transisi "Sudah Diambil" menggantikan "mark sold".
- **Surface kolom baru** (FR-ADM-GDS-01/02): response publik & admin, listing admin, dan **Export CSV** memuat `jenis_barang`, `jumlah`, `lokasi_pengambilan`, `status` ketersediaan, `foto_urls`.
- **Input create/update** disesuaikan: hapus `harga`/`kondisi`; tambah field baru. Endpoint create dari sisi pengguna (prasyarat mobile) mengikuti model baru.

## Capabilities

### New Capabilities
- `barang-bekas-gratis-model`: representasi iklan barang bekas/donasi gratis dengan jenis barang, jumlah, lokasi pengambilan, dan status ketersediaan (Tersedia/Sudah Diambil), beserta surfacing kolom pada listing admin & CSV.

## Impact

- **Kode**: `rust-services/iklan-barang-bekas-service/` — `domain/entity.rs` (field), `application/dto.rs` (Create/Update/Response/Admin), `application/service.rs` (mapping + transisi status), `infrastructure/pg_repository.rs` (INSERT/SELECT kolom baru), `interface/handlers.rs` (CSV header + baris). Endpoint `mark_sold` → diganti `mark_taken` (`PATCH /{id}/taken`).
- **Basis data** (schema iklan barang bekas): migrasi — `DROP COLUMN harga, kondisi`; `ADD COLUMN jenis_barang TEXT CHECK (jenis_barang IN ('bekas','baru'))`, `jumlah INT CHECK (jumlah >= 1)`, `lokasi_pengambilan TEXT`; ganti `is_sold BOOLEAN` → `availability_status TEXT DEFAULT 'tersedia' CHECK (availability_status IN ('tersedia','sudah_diambil'))`. Migrasi aditif+destruktif; **didokumentasikan, tidak dieksekusi** pada change ini.
- **API**: response & CSV memuat kolom baru; `PATCH /barang/{id}/taken` menggantikan `/sold`.
- **Dependensi**: `extend-iklan-moderation` (moderasi/suspend/listing/CSV existing — change ini menyelaraskan field yang di-surface).
- **Lintas-aplikasi**: **Rejki Mobile** form create iklan barang bekas harus disesuaikan (hapus harga; tambah jenis/jumlah/lokasi pengambilan) — prasyarat ([prd-mobile.md](../../../docs/prd/prd-mobile.md)).

## Non-Goals

- UI dashboard (Vue) — `add-rejki-web-dashboard`.
- Field iklan Pekerja/Pekerjaan (Pengalaman/Jam Kerja/Cara Menghubungi) — open question terpisah ([dashboard-gap-analysis.md](../../../docs/dashboard-gap-analysis.md) §5).
- Sistem reservasi/antrian pengambilan barang — di luar scope.
- Migrasi data historis harga (data existing): strategi backfill didokumentasikan, bukan dieksekusi.
