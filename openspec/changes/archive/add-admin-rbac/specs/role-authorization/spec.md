## ADDED Requirements

### Requirement: Identitas memiliki role yang extensible

Sistem SHALL menyimpan `role` pada identitas pengguna dengan nilai awal `user` dan `admin`, dan SHALL menetapkan `user` sebagai default. Skema `role` SHALL dirancang agar penambahan nilai role baru di masa depan bersifat aditif (tidak memaksa perubahan breaking pada konsumen yang sudah ada).

#### Scenario: Pengguna baru memperoleh role default
- **WHEN** pengguna terdaftar melalui registrasi publik
- **THEN** identitasnya memiliki `role=user`

#### Scenario: Klaim token memuat role
- **WHEN** sistem menerbitkan token untuk pengguna
- **THEN** klaim token memuat `role` pengguna tersebut

### Requirement: Otorisasi admin default-deny pada endpoint admin

Sistem SHALL melindungi seluruh endpoint di namespace admin dengan otorisasi peran yang menolak akses secara default. Akses SHALL diberikan hanya jika klaim token memuat `role=admin`. Permintaan tanpa klaim role yang valid SHALL ditolak dengan galat otorisasi.

#### Scenario: Permintaan admin oleh non-admin ditolak
- **WHEN** pemegang token ber-`role=user` mengakses endpoint admin
- **THEN** sistem menolak dengan galat otorisasi (kode mesin yang menandakan akun bukan admin)

#### Scenario: Permintaan admin tanpa klaim role ditolak (default-deny)
- **WHEN** permintaan ke endpoint admin tidak membawa klaim role yang valid
- **THEN** sistem menolak akses secara default

#### Scenario: Permintaan admin oleh admin diizinkan
- **WHEN** pemegang token ber-`role=admin` mengakses endpoint admin
- **THEN** sistem mengizinkan permintaan diproses
