## ADDED Requirements

### Requirement: Registrasi dengan email, telepon, password, dan persetujuan T&C

Sistem SHALL menerima registrasi pengguna dengan field `email`, `phone`, `password`, dan `tos_accepted`. Registrasi SHALL ditolak bila `tos_accepted` bukan `true`. Nomor `phone` SHALL disimpan dalam format E.164 tanpa verifikasi SMS. Saat persetujuan diterima, sistem SHALL mencatat `tos_accepted_at` (waktu) dan `tos_version` (versi dokumen T&C yang berlaku).

#### Scenario: Registrasi berhasil dengan data lengkap
- **WHEN** pengguna mengirim `email` valid, `phone` E.164 valid, `password` memenuhi kebijakan, dan `tos_accepted = true`
- **THEN** sistem membuat akun berstatus `pending_verification`, mencatat `tos_accepted_at` dan `tos_version`, menyimpan `phone` (terenkripsi at-rest), lalu memicu pengiriman OTP `register` ke email

#### Scenario: Persetujuan T&C tidak diberikan
- **WHEN** pengguna mengirim data registrasi dengan `tos_accepted = false` atau tanpa field tersebut
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` yang menandai field `tos_accepted`

#### Scenario: Nomor telepon bukan format E.164
- **WHEN** pengguna mengirim `phone` yang tidak sesuai format E.164
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` yang menandai field `phone`

### Requirement: Respons registrasi anti-enumeration

Sistem SHALL memberikan respons registrasi yang tidak membocorkan apakah sebuah email sudah terdaftar. Untuk email yang sudah terdaftar, sistem SHALL TIDAK mengembalikan galat yang membedakannya dari registrasi baru, dan SHALL TIDAK membuat akun duplikat.

#### Scenario: Registrasi dengan email yang sudah terdaftar
- **WHEN** pengguna mengirim registrasi dengan email yang sudah ada di sistem
- **THEN** sistem mengembalikan respons sukses yang identik dengan registrasi baru tanpa menyatakan bahwa email sudah terdaftar, dan tidak membuat akun kedua

#### Scenario: Email sudah terdaftar tetapi belum terverifikasi
- **WHEN** email yang dikirim sudah ada namun masih berstatus `pending_verification`
- **THEN** sistem memicu pengiriman ulang OTP `register` ke email tersebut dan membalas respons sukses generik

### Requirement: Kebijakan kekuatan password

Sistem SHALL menerima password hanya bila minimal 8 karakter dan mengandung minimal satu huruf kapital, satu angka, dan satu karakter spesial. Kebijakan ini SHALL berlaku pada registrasi, reset password, dan perubahan password.

#### Scenario: Password tidak memenuhi kebijakan
- **WHEN** pengguna mengirim password yang kurang dari 8 karakter atau tidak memuat salah satu dari huruf kapital, angka, atau karakter spesial
- **THEN** sistem menolak dengan `422` dan kode `VALIDATION` yang menandai field `password` beserta aturan yang dilanggar

#### Scenario: Password memenuhi kebijakan
- **WHEN** pengguna mengirim password dengan minimal 8 karakter yang memuat huruf kapital, angka, dan karakter spesial
- **THEN** sistem menerima password dan menyimpannya sebagai hash (bukan plaintext)
