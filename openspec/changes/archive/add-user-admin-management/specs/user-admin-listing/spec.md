## ADDED Requirements

### Requirement: Daftar pengajuan verifikasi KYC untuk admin

Sistem SHALL menyediakan endpoint admin yang mengembalikan daftar pengajuan verifikasi data diri (KYC) dengan kolom sesuai User Story: ID, Nama, Tingkat Pendidikan, Jenis Kelamin, Tanggal Lahir, Alamat Domisili, identitas wilayah (Kelurahan hingga Negara), dan Status Verifikasi. Endpoint SHALL diproteksi sehingga hanya pembawa token ber-role admin yang dapat mengaksesnya. NIK SHALL TIDAK disertakan dalam daftar; bila ditampilkan, hanya dalam bentuk ter-mask. Daftar SHALL mendukung pencarian, pemfilteran/pengurutan berdasarkan status, dan paginasi sisi server.

#### Scenario: Admin melihat daftar pengajuan KYC

- **WHEN** admin meminta daftar pengajuan KYC
- **THEN** sistem mengembalikan data terpaginasi dengan kolom sesuai User Story dan NIK tidak ditampilkan penuh

#### Scenario: Non-admin ditolak

- **WHEN** pembawa token tanpa role admin meminta daftar pengajuan KYC
- **THEN** sistem menolak permintaan dengan galat otorisasi

#### Scenario: Admin mencari dan memfilter

- **WHEN** admin mengirim kata kunci pencarian dan filter status
- **THEN** sistem mengembalikan hanya pengajuan yang cocok, terurut dan terpaginasi sesuai parameter

### Requirement: Detail pengajuan KYC untuk admin

Sistem SHALL menyediakan endpoint admin yang mengembalikan detail satu pengajuan KYC untuk ditampilkan pada pop-up, dengan NIK dalam keadaan ter-mask secara default. Endpoint SHALL diproteksi role admin dan SHALL mengembalikan galat tidak-ditemukan untuk identitas yang tidak valid (mencegah kebocoran keberadaan data).

#### Scenario: Admin membuka detail pengajuan

- **WHEN** admin meminta detail satu pengajuan KYC yang ada
- **THEN** sistem menampilkan data diri lengkap dengan NIK ter-mask

#### Scenario: Identitas tidak valid

- **WHEN** admin meminta detail dengan identitas yang tidak ada
- **THEN** sistem mengembalikan galat tidak-ditemukan tanpa membocorkan keberadaan data

### Requirement: Ekspor CSV daftar pengajuan KYC

Sistem SHALL menyediakan endpoint admin yang mengekspor daftar pengajuan KYC dalam berkas CSV sesuai pencarian dan filter yang sedang aktif.

#### Scenario: Admin mengekspor daftar pengajuan

- **WHEN** admin meminta ekspor CSV dengan filter aktif
- **THEN** sistem mengembalikan berkas CSV berisi data sesuai filter
