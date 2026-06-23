## MODIFIED Requirements

### Requirement: Login admin terpisah dengan anti-enumeration penuh (termasuk timing)

Endpoint login admin (`POST /api/v1/auth/admin/login`) SHALL memverifikasi password TERLEBIH DAHULU sebelum pengecekan role dan status. Sistem SHALL menggunakan dummy bcrypt hash untuk email yang tidak terdaftar atau bukan admin, sehingga response time seragam (~250ms untuk semua kasus). Sistem SHALL mengembalikan HTTP 401 dengan pesan seragam "email atau password salah" untuk semua kegagalan.

#### Scenario: Login admin berhasil

- **WHEN** admin mengirim email + password yang valid dan role=admin serta status=active
- **THEN** sistem menerbitkan access+refresh token dengan klaim role=admin, mengembalikan HTTP 200, dan mencatat audit log admin login

#### Scenario: Login admin gagal — email tidak terdaftar

- **WHEN** email yang dikirim tidak terdaftar di database
- **THEN** sistem menjalankan bcrypt verify dengan dummy hash (~250ms), lalu mengembalikan HTTP 401 "email atau password salah" — response time identik dengan case lain

#### Scenario: Login admin gagal — bukan admin

- **WHEN** email terdaftar tetapi user bukan admin (role=user), password benar atau salah
- **THEN** sistem menjalankan bcrypt verify penuh, lalu mengembalikan HTTP 401 "email atau password salah" — response time identik dengan case lain

#### Scenario: Login admin gagal — admin tidak aktif

- **WHEN** email milik admin tetapi status bukan active (suspended, dll)
- **THEN** sistem menjalankan bcrypt verify penuh, lalu mengembalikan HTTP 401 "email atau password salah" — response time identik
