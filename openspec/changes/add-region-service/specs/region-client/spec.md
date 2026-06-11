## ADDED Requirements

### Requirement: Antarmuka RegionClient untuk konsumen domain lain

Sistem SHALL menyediakan antarmuka `RegionClient` yang dapat dipanggil secara in-process oleh domain lain. Antarmuka ini SHALL menyediakan operasi lookup daftar per tingkat dan pengambilan satu wilayah berdasarkan pengenal. Domain lain SHALL mengakses data wilayah hanya melalui antarmuka ini, bukan dengan mengimpor implementasi internal region-service.

#### Scenario: Domain lain mengambil satu wilayah
- **WHEN** domain lain memanggil operasi pengambilan wilayah dengan pengenal yang ada
- **THEN** sistem mengembalikan data wilayah tersebut beserta tingkatnya

#### Scenario: Pengenal wilayah tidak ditemukan
- **WHEN** domain lain meminta wilayah dengan pengenal yang tidak ada
- **THEN** sistem mengembalikan hasil "tidak ditemukan" tanpa menimbulkan galat server

### Requirement: Validasi rantai wilayah berjenjang

Sistem SHALL menyediakan operasi `validate_chain(province_id, regency_id, district_id, village_id)` yang mengembalikan benar hanya jika seluruh tingkat saling konsisten secara berjenjang (kelurahan berinduk pada kecamatan, kecamatan pada kabupaten/kota, kabupaten/kota pada provinsi). Konsumen (mis. user-service saat menyimpan alamat KYC) SHALL memvalidasi rantai melalui operasi ini sebelum menyimpan.

#### Scenario: Rantai wilayah valid
- **WHEN** `validate_chain` dipanggil dengan kombinasi provinsi, kabupaten/kota, kecamatan, dan kelurahan yang konsisten
- **THEN** sistem mengembalikan benar

#### Scenario: Rantai wilayah tidak konsisten
- **WHEN** `validate_chain` dipanggil dengan kelurahan yang tidak berinduk pada kecamatan yang diberikan (atau ketidakcocokan induk pada tingkat mana pun)
- **THEN** sistem mengembalikan salah

#### Scenario: Pengenal pada rantai tidak ada
- **WHEN** `validate_chain` dipanggil dan salah satu pengenal tidak ada di data wilayah
- **THEN** sistem mengembalikan salah
