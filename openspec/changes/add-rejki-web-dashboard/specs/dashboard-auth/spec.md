## ADDED Requirements

### Requirement: Halaman login admin

Sistem klien SHALL menyediakan halaman login dengan form email dan password yang mengirim kredensial ke endpoint login admin. Halaman SHALL menangani galat kredensial tanpa membocorkan detail akun. Setelah login berhasil, sistem klien SHALL menyimpan token dan mengalihkan ke halaman utama dashboard.

#### Scenario: Admin login berhasil
- **WHEN** admin mengirim email dan password yang benar
- **THEN** token disimpan, profil dimuat, dan admin dialihkan ke halaman dashboard

#### Scenario: Kredensial salah
- **WHEN** admin mengirim kredensial yang salah
- **THEN** sistem klien menampilkan pesan galat generik tanpa membocorkan detail akun

### Requirement: Otorisasi klien dan route guard

Sistem klien SHALL memeriksa keberadaan token dan role admin sebelum mengizinkan akses ke rute dashboard. Tanpa token yang valid, pengguna SHALL dialihkan ke halaman login. Tanpa role admin, akses SHALL ditolak.

#### Scenario: Pengguna tanpa token dialihkan ke login
- **WHEN** pengguna tanpa token mencoba mengakses rute dashboard
- **THEN** sistem klien mengalihkan ke halaman login

#### Scenario: Pengguna non-admin ditolak
- **WHEN** pengguna dengan token non-admin mencoba mengakses rute dashboard
- **THEN** sistem klien menolak akses dan menampilkan pemberitahuan

### Requirement: Header profil dan logout

Sistem klien SHALL menampilkan foto profil, nama, dan role admin pada header. Menu yang muncul saat diklik SHALL menyediakan opsi logout yang menghapus sesi dan mengalihkan ke halaman login.

#### Scenario: Admin logout
- **WHEN** admin memilih logout dari menu header
- **THEN** sesi dihapus, token dihapus, dan admin dialihkan ke halaman login
