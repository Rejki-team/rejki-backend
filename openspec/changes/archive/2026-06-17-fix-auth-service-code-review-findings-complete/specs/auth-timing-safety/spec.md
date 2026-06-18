## ADDED Requirements

### Requirement: Constant-time authentication pada admin login

Sistem SHALL menjalankan bcrypt password verification PADA SEMUA jalur permintaan `admin_login`, termasuk untuk email yang tidak terdaftar atau bukan admin. Ini menghilangkan timing side-channel yang dapat membedakan akun admin dari non-admin berdasarkan response time. Sistem SHALL menggunakan dummy bcrypt hash untuk jalur yang tidak memiliki user valid.

#### Scenario: Admin login — semua jalur memiliki response time yang seragam

- **WHEN** sebuah permintaan `POST /admin/login` diterima
- **THEN** sistem menjalankan bcrypt password verification dengan durasi yang sama (~250ms) untuk semua kasus berikut: email tidak terdaftar, email non-admin, admin dengan password salah, admin dengan password benar

#### Scenario: Dummy bcrypt hash untuk email tidak terdaftar

- **WHEN** `find_by_email` mengembalikan `None`
- **THEN** sistem menggunakan dummy bcrypt hash konstan untuk password verification, lalu mengembalikan error "email atau password salah"

#### Scenario: Dummy bcrypt hash untuk user non-admin

- **WHEN** `find_by_email` mengembalikan user dengan role selain admin
- **THEN** sistem tetap menjalankan bcrypt password verification dengan hash user tersebut, lalu mengembalikan error "email atau password salah" (tanpa membocorkan bahwa user bukan admin)

### Requirement: Anti-enumeration pada login reguler via HTTP 401 seragam

Sistem SHALL mengembalikan HTTP 401 `UNAUTHORIZED` untuk SEMUA kegagalan pada endpoint `/login` — termasuk email tidak terdaftar, password salah, akun tidak aktif, akun ditangguhkan. Tidak boleh ada perbedaan HTTP status code atau pesan error yang dapat digunakan untuk enumerasi akun. Sistem SHALL TIDAK mengembalikan detail error internal (seperti "akun belum diverifikasi") pada response body.

#### Scenario: Login gagal — selalu 401

- **WHEN** permintaan login gagal karena alasan apapun (kredensial salah, akun tidak ditemukan, akun ditangguhkan, akun belum aktif)
- **THEN** sistem mengembalikan HTTP 401 dengan body seragam `{"error": "UNAUTHORIZED", "message": "unauthorized"}`

#### Scenario: Admin login gagal — selalu 401

- **WHEN** permintaan admin login gagal karena alasan apapun
- **THEN** sistem mengembalikan HTTP 401 dengan body seragam tanpa membocorkan detail
