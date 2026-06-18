## 1. Scaffold Crate & Workspace

- [x] 1.1 Buat crate `rust-services/region-service/` (struktur 4 layer: domain, application, infrastructure, interface) mengikuti pola service lain
- [x] 1.2 Buat crate `rust-services/region-service-client/` (trait + tipe publik)
- [x] 1.3 Daftarkan kedua crate sebagai member di workspace `rust-services/Cargo.toml`
- [x] 1.4 Tambah keduanya ke daftar COPY/dummy di `rust-services/Dockerfile` agar build cache tetap benar

## 2. Migrasi Struktur (schema `region`)

- [x] 2.1 Buat schema `region` dan tabel `province (id TEXT PK, name)` (migrasi up + down)
- [x] 2.2 Buat tabel `regency (id TEXT PK, province_id FK→province, name)` + index `province_id`
- [x] 2.3 Buat tabel `district (id TEXT PK, regency_id FK→regency, name)` + index `regency_id`
- [x] 2.4 Buat tabel `village (id TEXT PK, district_id FK→district, name)` + index `district_id`
- [x] 2.5 Tambahkan pencatatan versi dataset (tabel `region_dataset_version` atau metadata setara)
- [x] 2.6 Verifikasi migrasi naik & turun bersih pada DB test

## 3. Seeding Data Wilayah

- [x] 3.1 Siapkan dataset wilayah dari sumber Kepmendagri terbaru (mis. cahyadsn/wilayah) dalam bentuk yang dapat di-seed
- [x] 3.2 Implementasi proses seed idempoten (terpisah dari migrasi struktur) yang mengisi 4 tabel + mencatat versi dataset
- [x] 3.3 Verifikasi jumlah baris per tingkat masuk akal dan rantai induk konsisten (uji sampling)

## 4. Domain & Repository (region-service)

- [x] 4.1 Definisikan entity domain `Region` + enum `RegionLevel` (province/regency/district/village)
- [x] 4.2 Implementasi repository read: list per tingkat berdasarkan induk, get satu wilayah by id
- [x] 4.3 Implementasi `validate_chain(province_id, regency_id, district_id, village_id)` di repository/application
- [x] 4.4 Unit test validate_chain: rantai valid → true; induk tidak cocok → false; id tidak ada → false

## 5. API Cascading (spec: region-lookup-api)

- [x] 5.1 Handler `GET /api/v1/regions/provinces`
- [x] 5.2 Handler `GET /api/v1/regions/regencies?province_id=`
- [x] 5.3 Handler `GET /api/v1/regions/districts?regency_id=`
- [x] 5.4 Handler `GET /api/v1/regions/villages?district_id=`
- [x] 5.5 Pastikan envelope `ApiResponse`, parent tidak dikenal → daftar kosong, propagasi `request_id`; tidak ada endpoint tulis
- [x] 5.6 Integration test: tiap level mengembalikan data sesuai induk; parent tak dikenal → daftar kosong

## 6. RegionClient (spec: region-client)

- [x] 6.1 Definisikan trait `RegionClient` di `region-service-client`: list per level, get_region(id), validate_chain(...)
- [x] 6.2 Implementasi `RegionInProcessClient` di region-service (memanggil application service)
- [x] 6.3 Integration test in-process: get_region ada/tidak ada; validate_chain valid/tidak

## 7. Wiring & Finalisasi

- [x] 7.1 Wire router region-service di `rejki-app` di bawah prefix `/api/v1/regions`
- [x] 7.2 Wire `RegionInProcessClient` sebagai dependency yang dapat di-inject ke konsumen (disiapkan untuk user-service)
- [x] 7.3 Jalankan `cargo fmt`, `clippy -D warnings`, dan seluruh test hingga hijau
- [x] 7.4 Catat versi dataset wilayah pada dokumentasi agar dapat di-update berkala
