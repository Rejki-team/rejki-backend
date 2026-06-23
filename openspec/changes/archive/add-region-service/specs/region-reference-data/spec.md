## ADDED Requirements

### Requirement: Model data wilayah empat tingkat berjenjang

Sistem SHALL menyimpan data wilayah administratif Indonesia dalam empat tingkat: provinsi, kabupaten/kota, kecamatan, dan kelurahan/desa. Setiap entri SHALL menggunakan kode wilayah resmi sebagai pengenal dan SHALL merujuk pengenal induknya (kecuali provinsi yang merupakan tingkat teratas). Relasi induk SHALL berada dalam satu schema tanpa kunci asing lintas-schema.

#### Scenario: Setiap tingkat memiliki induk yang benar
- **WHEN** sebuah entri kabupaten/kota disimpan
- **THEN** entri tersebut merujuk pengenal provinsi yang ada; demikian pula kecamatan merujuk kabupaten/kota, dan kelurahan merujuk kecamatan

#### Scenario: Pengenal wilayah unik per tingkat
- **WHEN** data wilayah dimuat
- **THEN** setiap pengenal wilayah bersifat unik dan dapat ditelusuri ke induknya hingga tingkat provinsi

### Requirement: Seeding dari dataset resmi tanpa dependensi runtime eksternal

Sistem SHALL mengisi data wilayah melalui proses seeding dari dataset Kepmendagri terbaru ke dalam basis data, terpisah dari migrasi struktur tabel. Sistem SHALL TIDAK memanggil API pihak ketiga saat runtime untuk melayani permintaan wilayah. Sistem SHALL mencatat versi dataset yang dipakai agar dapat diperbarui berkala.

#### Scenario: Data tersedia dari basis data lokal
- **WHEN** permintaan data wilayah dilayani
- **THEN** sistem membaca dari basis data lokal tanpa memanggil layanan eksternal

#### Scenario: Versi dataset tercatat
- **WHEN** data wilayah di-seed
- **THEN** sistem mencatat identitas/versi dataset (mis. rujukan Kepmendagri) yang digunakan
