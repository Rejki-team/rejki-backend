## Why

Data wilayah administratif Indonesia (provinsi → kabupaten/kota → kecamatan → kelurahan) dibutuhkan oleh banyak domain: user-service untuk alamat KYC, keempat service iklan untuk `lokasi`, dan analytics CEO untuk canvassing per wilayah. Saat ini tidak ada sumber wilayah terstruktur — `lokasi` di iklan masih teks bebas dan KYC belum punya referensi. Menaruh data ini di salah satu service (mis. user-service) akan memaksa domain lain mengimpornya hanya untuk validasi wilayah — kebocoran batas domain. Keputusan brainstorming K13 (revisi) menetapkan data wilayah sebagai domain mandiri.

## What Changes

- **Domain wilayah mandiri** (`region-service`) memiliki data referensi 4 tingkat: provinsi, kabupaten/kota, kecamatan, kelurahan/desa, dengan kode wilayah resmi sebagai primary key dan relasi berjenjang `parent_id`.
- **Seeding dari dataset Kepmendagri terbaru** ke dalam PostgreSQL (schema `region`), bukan memanggil API pihak ketiga saat runtime — agar tidak ada dependensi jaringan eksternal dan konsisten. Versi dataset dicatat agar dapat di-update berkala.
- **API cascading bertahap** (`/api/v1/regions/*`): daftar provinsi, lalu per level berdasarkan parent terpilih — mendukung dropdown bertingkat di klien (Flutter) tanpa memuat seluruh data ke klien.
- **Trait `RegionClient`** (in-process) untuk konsumen domain lain: lookup per level, ambil satu wilayah, dan **validasi rantai wilayah** (memastikan kelurahan benar milik kecamatan, kecamatan milik kota, kota milik provinsi).
- **Negara**: kolom negara default `ID`; daftar negara penuh belum didukung pada tahap ini.

## Capabilities

### New Capabilities
- `region-reference-data`: model data wilayah 4 tingkat, kode wilayah, relasi berjenjang, dan seeding dari dataset resmi.
- `region-lookup-api`: endpoint cascading `/api/v1/regions/*` untuk penelusuran bertahap per level.
- `region-client`: antarmuka in-process `RegionClient` untuk lookup & validasi rantai wilayah oleh domain lain.

### Modified Capabilities
<!-- Tidak ada: belum ada spec ter-arsip di openspec/specs/. Region adalah domain baru sepenuhnya. -->

## Impact

- **Kode**: crate baru `rust-services/region-service/` (domain, application, infrastructure, interface) dan `rust-services/region-service-client/` (trait `RegionClient` + tipe), menambah 1 pasang member ke workspace; wiring router & client di `rust-services/rejki-app/`.
- **Basis data**: schema baru `region` dengan tabel `province`, `regency`, `district`, `village` (kode wilayah sebagai PK, `parent_id` ber-FK dalam schema yang sama). Tidak ada FK lintas-schema. Perlu mekanisme seed terpisah dari migrasi struktur.
- **API**: endpoint publik baru `/api/v1/regions/provinces`, `/regions/regencies`, `/regions/districts`, `/regions/villages` (read-only, query by parent).
- **Konsumen hilir**: user-service (KYC, proposal terpisah) memanggil `RegionClient.validate_chain(...)`; iklan-service & analytics CEO sebagai konsumen masa depan (penyelarasan `lokasi` iklan ditunda, di luar scope).
- **Standar**: mengikuti envelope `ApiResponse`, error RFC 9457-inspired, prefix `/api/v1/`, snake_case DB, propagasi `request_id`.

## Non-Goals

- Data KYC/profil pengguna (proposal user-service terpisah).
- Penyelarasan field `lokasi` di service iklan dari teks bebas ke `region_id` (ditunda; brainstorming iklan/CEO).
- Daftar negara penuh (ISO 3166) — hanya default `ID` pada tahap ini.
- Pembuatan/penyuntingan data wilayah lewat API (data bersifat referensi read-only, di-seed; perubahan via proses seed/migrasi, bukan endpoint publik).
- Geocoding, peta, atau koordinat lintang/bujur.
