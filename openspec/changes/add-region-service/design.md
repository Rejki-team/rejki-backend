## Context

Rejki adalah modular monolith Rust: setiap domain berupa crate `[lib]` (service + service-client), seluruhnya dikompilasi menjadi satu binary `rejki-app`. Pola yang sudah ada: `auth-service`/`auth-service-client` dengan `AuthInProcessClient` di-wire di `rejki-app`. Data wilayah Indonesia dibutuhkan lintas domain (user/KYC, iklan, analytics CEO). Keputusan brainstorming K13 (revisi) menetapkan wilayah sebagai domain mandiri, bukan menumpang di user-service. Tersedia dataset open-source gratis sesuai Kepmendagri terbaru (mis. cahyadsn/wilayah) sebagai sumber seed.

## Goals / Non-Goals

**Goals:**
- Menyediakan data wilayah 4 tingkat sebagai domain mandiri read-only.
- Endpoint cascading untuk klien dan `RegionClient` in-process untuk domain lain.
- Validasi rantai wilayah agar alamat KYC konsisten.
- Tanpa dependensi jaringan eksternal saat runtime.

**Non-Goals:**
- Mutasi data wilayah via API, daftar negara penuh, penyelarasan `lokasi` iklan, geocoding/koordinat.

## Decisions

### D1 — Crate baru `region-service` + `region-service-client` (K13)
Mengikuti pola domain lain. `region-service` berisi data & logika lookup/validasi; `region-service-client` mengekspos `RegionClient` trait + tipe (mis. `Region`, `RegionLevel`). Alternatif "crate library bersama tanpa service" ditolak karena klien Flutter butuh endpoint cascading HTTP dan banyak konsumen lebih bersih dilayani satu pemilik domain ber-API. Alternatif "taruh di user-service" ditolak (kebocoran batas domain).

### D2 — Schema `region` dengan kode wilayah sebagai PK
Tabel `province`, `regency`, `district`, `village`. PK = kode wilayah resmi bertipe `TEXT` (mis. `32`, `32.04`, `32.04.05`, `32.04.05.2001`); kolom induk ber-FK dalam schema yang sama (`regency.province_id → province.id`, dst.). Memakai kode resmi sebagai PK memudahkan validasi rantai berbasis prefix dan stabil lintas update dataset. Index pada kolom induk untuk lookup cascading cepat. Tidak ada FK lintas-schema (konsumen menyimpan kode sebagai nilai, bukan FK).

### D3 — Seeding terpisah dari migrasi struktur
Migrasi hanya membuat struktur tabel + index. Pengisian data dilakukan lewat proses seed terpisah (mis. file SQL data / perintah seed) yang mengambil dari dataset Kepmendagri. Versi dataset dicatat (mis. tabel `region_dataset_version` atau metadata) agar dapat di-refresh. Alternatif "data di dalam file migrasi" dihindari agar update dataset tidak mengubah riwayat migrasi struktur.

### D4 — `RegionClient.validate_chain` sebagai operasi tunggal
Validasi rantai diekspos sebagai satu operasi agar konsumen (user-service) cukup satu panggilan in-process sebelum menyimpan alamat KYC. Implementasi memeriksa keberadaan tiap pengenal dan konsistensi induk. Karena PK memakai kode berprefiks, sebagian pemeriksaan dapat dipercepat, namun kebenaran tetap diverifikasi terhadap data (bukan hanya prefix string).

### D5 — Endpoint read-only mengikuti standar Phase 1.5
`GET /api/v1/regions/{provinces|regencies|districts|villages}` dengan query parameter induk; envelope `ApiResponse`, error RFC 9457-inspired, propagasi `request_id`. Parent tidak dikenal → daftar kosong (bukan galat). Tidak ada endpoint tulis.

## Risks / Trade-offs

- **Ukuran data besar (puluhan ribu kelurahan)** → jangan kirim semua ke klien; endpoint selalu memfilter per induk. Index pada kolom induk wajib agar query cepat.
- **Pembaruan dataset Kepmendagri berkala** → mitigasi: proses seed idempoten + catatan versi; perubahan kode wilayah resmi jarang, tetapi perlu strategi update yang tidak merusak referensi tersimpan di profil pengguna.
- **Kode wilayah tersimpan di domain lain tanpa FK** → integritas dijaga lewat `validate_chain` saat tulis di sisi konsumen, bukan oleh constraint DB lintas-schema.
- **Menambah 2 crate ke workspace** → konsisten dengan pola yang ada; overhead kecil dibanding manfaat batas domain yang jelas.

## Migration Plan

1. Buat crate `region-service` + `region-service-client`, daftarkan ke workspace `Cargo.toml`.
2. Migrasi struktur: schema `region` + tabel `province/regency/district/village` + index induk (up + down).
3. Siapkan proses seed dari dataset Kepmendagri + pencatatan versi dataset.
4. Implementasi repository lookup + `validate_chain`, application service, dan handler endpoint cascading.
5. Implementasi `RegionInProcessClient` dan wiring router + client di `rejki-app`.
6. Rollback: migrasi turun menghapus schema `region`; tidak ada dampak lintas-schema karena konsumen hanya menyimpan nilai kode.

## Resolved Questions

- **Sumber dataset**: dataset open-source sesuai Kepmendagri terbaru (mis. cahyadsn/wilayah) — versi pasti ditetapkan saat seeding (parameter implementasi).
- **Negara**: hanya default `ID` pada tahap ini; daftar negara penuh di luar scope.
