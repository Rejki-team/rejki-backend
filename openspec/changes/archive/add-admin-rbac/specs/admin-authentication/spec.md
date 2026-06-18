## ADDED Requirements

### Requirement: Login admin dengan email dan password tanpa OTP

Sistem SHALL menyediakan endpoint login admin terpisah yang menerima email dan password, memverifikasi kredensial, dan hanya berhasil bila akun memiliki `role = admin` dan `status = active`. Login admin SHALL menerbitkan access dan refresh token RS256 yang memuat klaim `role`. Login admin SHALL TIDAK mewajibkan OTP.

#### Scenario: Admin login berhasil
- **WHEN** akun ber-`role=admin` dan `status=active` mengirim email dan password yang benar ke endpoint login admin
- **THEN** sistem menerbitkan access dan refresh token yang memuat klaim `role=admin`

#### Scenario: Kredensial salah ditolak tanpa membocorkan keberadaan akun
- **WHEN** email atau password salah dikirim ke endpoint login admin
- **THEN** sistem menolak dengan galat seragam tanpa membocorkan apakah email terdaftar

#### Scenario: Pengguna non-admin ditolak di endpoint admin
- **WHEN** akun ber-`role=user` mengirim kredensial yang benar ke endpoint login admin
- **THEN** sistem menolak login dan tidak menerbitkan token admin

### Requirement: Akun admin disediakan oleh DBA tanpa self-register

Sistem SHALL menyediakan akun admin melalui proses seed/injeksi DBA, bukan melalui pendaftaran mandiri. Sistem SHALL TIDAK menyediakan endpoint registrasi admin. Akun admin yang di-inject SHALL berstatus `active` dan tidak diwajibkan menjalani alur verifikasi KYC.

#### Scenario: Akun admin dibuat via seed
- **WHEN** DBA menjalankan proses seed akun admin dengan email dan password
- **THEN** akun tercipta dengan `role=admin` dan `status=active` serta dapat login melalui endpoint login admin

#### Scenario: Tidak ada pendaftaran admin mandiri
- **WHEN** pihak mana pun mencoba mendaftarkan akun ber-`role=admin` melalui alur registrasi publik
- **THEN** sistem tidak membuat akun admin (registrasi publik hanya menghasilkan `role=user`)

### Requirement: Profil menyertakan role untuk tampilan dashboard

Sistem SHALL menyertakan `role` pada respons profil pengguna saat ini sehingga klien dashboard dapat menampilkan foto profil, nama, dan role admin.

#### Scenario: Profil admin menampilkan role
- **WHEN** admin terautentikasi memanggil endpoint profil "saya"
- **THEN** respons memuat `role=admin` beserta data profil (nama, avatar)
