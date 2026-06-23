## ADDED Requirements

### Requirement: Pengiriman data diri KYC yang lengkap

Sistem SHALL menerima data diri KYC berisi: nama lengkap, NIK, tingkat pendidikan, jenis kelamin, tanggal lahir, alamat domisili, negara (default `ID`), provinsi, kabupaten/kota, kecamatan, dan kelurahan. Sistem SHALL menolak pengiriman yang tidak lengkap pada field wajib dengan galat validasi.

#### Scenario: Data diri lengkap diterima
- **WHEN** pengguna mengirim seluruh field data diri yang diwajibkan dengan nilai valid
- **THEN** sistem menyimpan data diri dan melanjutkan ke alur verifikasi

#### Scenario: Data diri tidak lengkap
- **WHEN** pengguna mengirim data diri dengan salah satu field wajib kosong/tidak valid
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` yang menandai field bermasalah

### Requirement: NIK disimpan terenkripsi dan tidak dapat diubah

Sistem SHALL menyimpan NIK dalam bentuk terenkripsi at-rest (AES-256-GCM) dan menyimpan bentuk ter-mask (mis. 4 digit terakhir) untuk tampilan. Setelah NIK dikirim, sistem SHALL TIDAK mengizinkan perubahan NIK melalui alur self-service; perubahan hanya melalui proses admin khusus.

#### Scenario: NIK disimpan terenkripsi
- **WHEN** data diri dengan NIK disimpan
- **THEN** nilai NIK tersimpan dalam bentuk terenkripsi, dan tersedia bentuk ter-mask untuk tampilan

#### Scenario: Upaya mengubah NIK ditolak
- **WHEN** pengguna mencoba mengubah NIK setelah sebelumnya terkirim
- **THEN** sistem menolak perubahan tersebut melalui alur self-service

### Requirement: Validasi rantai wilayah melalui RegionClient

Sistem SHALL memvalidasi konsistensi provinsi → kabupaten/kota → kecamatan → kelurahan melalui `RegionClient.validate_chain` sebelum menyimpan data diri. Pengiriman dengan rantai wilayah tidak konsisten SHALL ditolak.

#### Scenario: Rantai wilayah valid
- **WHEN** data diri dikirim dengan kombinasi wilayah yang konsisten menurut `RegionClient.validate_chain`
- **THEN** sistem menerima dan menyimpan data wilayah

#### Scenario: Rantai wilayah tidak konsisten
- **WHEN** data diri dikirim dengan kelurahan/kecamatan/kabupaten yang tidak konsisten dengan induknya
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` pada field wilayah
