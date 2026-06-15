## Context

Entity `IklanBarangBekas` saat ini bermodel jual-beli (`harga`, `kondisi`, `is_sold`)
([entity.rs:40-53](../../../rust-services/iklan-barang-bekas-service/src/domain/entity.rs#L40)).
Moderasi, suspend, listing admin, dan CSV sudah ada via `extend-iklan-moderation`. Change ini
**menyelaraskan model data** ke "gratis/donasi" sesuai User Story, lalu memastikan kolom baru
ikut di-surface pada listing admin & CSV yang sudah ada.

## Goals / Non-Goals

**Goals:** model gratis (jenis_barang, jumlah, lokasi_pengambilan); status ketersediaan Tersedia/Sudah Diambil; surface kolom di response/listing/CSV.

**Non-Goals:** reservasi/antrian; field iklan pekerja/pekerjaan; eksekusi migrasi; UI Vue.

## Decisions

### D1 — Hapus harga & kondisi; tambah jenis_barang, jumlah, lokasi_pengambilan
Nama menu "Gratis" + User Story tidak menyebut harga sama sekali; kolom yang diminta adalah Jenis Barang
(Bekas/Baru), Jumlah, Lokasi Pengambilan. Maka `harga`/`kondisi` dihapus (bukan dipertahankan sebagai 0)
agar model jujur mencerminkan domain donasi dan tidak menyisakan field menyesatkan (Zero Hardcoded /
Zero field menyesatkan). `jenis_barang` pakai CHECK `bekas|baru` (extensible). `jumlah` INT ≥ 1.

### D2 — Status ketersediaan menggantikan is_sold
`is_sold: bool` → `availability_status: TEXT` dengan nilai `tersedia|sudah_diambil`. "Sudah Diambil"
adalah istilah donasi (barang diambil penerima), bukan "terjual". Status moderasi (`moderation_status`)
tetap terpisah; "Suspended" berasal dari moderasi, bukan dari availability. Dashboard menampilkan
gabungan: Tersedia / Sudah Diambil / Suspended.

### D3 — Transisi `mark_taken` menggantikan `mark_sold`
Endpoint `PATCH /barang/{id}/sold` ([interface/mod.rs:47](../../../rust-services/iklan-barang-bekas-service/src/interface/mod.rs#L47))
diganti `PATCH /barang/{id}/taken` yang men-set `availability_status='sudah_diambil'`. Hanya pemilik
(atau admin) yang boleh menandai. Idempoten.

### D4 — Surface kolom baru di listing admin & CSV existing
`AdminIklanBarangBekasResponse` + header/baris CSV di handler ([handlers.rs] pola seperti
[iklan-pekerja/handlers.rs:92-106](../../../rust-services/iklan-pekerja-service/src/interface/handlers.rs#L92))
ditambah `jenis_barang`, `jumlah`, `lokasi_pengambilan`, `availability_status`. Search/sort/pagination
existing (`extend-iklan-moderation`) tidak berubah strukturnya.

### D5 — Migrasi destruktif: didokumentasikan, strategi backfill
Migrasi `DROP COLUMN harga, kondisi` bersifat destruktif. Untuk data existing: dokumentasikan strategi
(mis. data uji/dev di-reset; bila ada data nyata, backfill `jenis_barang` default `bekas`, `jumlah=1`,
`lokasi_pengambilan` dari `lokasi`). Tidak dieksekusi pada change ini (spesifikasi-only sesuai keputusan).

## Risks / Trade-offs

- **Breaking schema & API**: klien lama (mobile) yang mengirim `harga`/`kondisi` akan gagal → mobile harus disesuaikan bersamaan (prasyarat lintas-app).
- **Kehilangan data harga historis**: diterima karena model bisnis berubah ke gratis; backfill didokumentasikan.
- **`lokasi` vs `lokasi_pengambilan`**: keduanya bisa hidup berdampingan (lokasi umum vs titik ambil) — default: pertahankan `lokasi` opsional, tambah `lokasi_pengambilan` wajib.

## Migration Plan

1. Migrasi DB (dokumentasi): drop harga/kondisi; add jenis_barang/jumlah/lokasi_pengambilan; is_sold→availability_status.
2. Entity + DTO (Create/Update/Response/Admin) disesuaikan.
3. Repository INSERT/SELECT kolom baru; service mapping + transisi `mark_taken`.
4. CSV header + baris memuat kolom baru.
5. Mobile form create disesuaikan (prasyarat) — lihat prd-mobile.
6. Uji: create dengan field baru; mark_taken; listing & CSV memuat kolom baru.

## Open Questions

- **`lokasi` lama**: dipertahankan atau dilebur ke `lokasi_pengambilan`? Default: pertahankan keduanya.
- **Backfill data nyata**: apakah ada data produksi? Bila tidak (greenfield/dev), migrasi langsung tanpa backfill.
