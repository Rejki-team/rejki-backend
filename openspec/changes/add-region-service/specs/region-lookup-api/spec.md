## ADDED Requirements

### Requirement: Penelusuran wilayah bertahap per tingkat

Sistem SHALL menyediakan endpoint read-only di bawah `/api/v1/regions/` yang mengembalikan daftar wilayah per tingkat: seluruh provinsi, kabupaten/kota berdasarkan provinsi induk, kecamatan berdasarkan kabupaten/kota induk, dan kelurahan berdasarkan kecamatan induk. Respons SHALL mengikuti envelope `ApiResponse` standar.

#### Scenario: Daftar provinsi
- **WHEN** klien memanggil `GET /api/v1/regions/provinces`
- **THEN** sistem mengembalikan daftar seluruh provinsi dalam envelope `ApiResponse`

#### Scenario: Daftar kabupaten/kota berdasarkan provinsi
- **WHEN** klien memanggil `GET /api/v1/regions/regencies?province_id=<id>` dengan provinsi yang ada
- **THEN** sistem mengembalikan kabupaten/kota yang berinduk pada provinsi tersebut

#### Scenario: Daftar kecamatan dan kelurahan berdasarkan induk
- **WHEN** klien memanggil `GET /api/v1/regions/districts?regency_id=<id>` lalu `GET /api/v1/regions/villages?district_id=<id>`
- **THEN** sistem mengembalikan kecamatan untuk kabupaten/kota tersebut, dan kelurahan untuk kecamatan tersebut

#### Scenario: Parameter induk tidak dikenal
- **WHEN** klien meminta daftar suatu tingkat dengan pengenal induk yang tidak ada
- **THEN** sistem mengembalikan daftar kosong dalam envelope `ApiResponse` (bukan galat server)

### Requirement: Endpoint wilayah bersifat read-only

Sistem SHALL TIDAK menyediakan endpoint publik untuk membuat, mengubah, atau menghapus data wilayah. Perubahan data wilayah SHALL dilakukan melalui proses seed/migrasi.

#### Scenario: Upaya mutasi via API ditolak
- **WHEN** klien mencoba membuat atau mengubah data wilayah melalui API publik
- **THEN** sistem tidak menyediakan operasi tersebut (tidak ada endpoint tulis wilayah)
