## MODIFIED Requirements

### Requirement: Kedaluwarsa, batas percobaan, dan rate limit OTP

Sistem SHALL menetapkan masa kedaluwarsa OTP yang pendek. Sistem SHALL membatasi jumlah percobaan verifikasi dan frekuensi permintaan OTP per pengguna untuk mencegah brute-force dan penyalahgunaan.

**PENTING:** Operasi `bump_otp_attempts` (UPDATE attempts + conditional DELETE) SHALL dibungkus dalam database transaction untuk mencegah race condition dengan `save_otp` (resend OTP). Tanpa transaction, `save_otp` yang terjadi antara UPDATE dan DELETE dapat mengakibatkan OTP baru yang legitimate dihapus.

**PENTING:** Redis rate limiter (`OtpRateLimiter`) SHALL selalu memanggil `EXPIRE` setiap kali `INCR` dijalankan — bukan hanya pada hit pertama — untuk menghindari key tanpa TTL yang menyebabkan permanent lockout.

#### Scenario: Melebihi batas percobaan verifikasi

- **WHEN** pengguna melampaui batas percobaan verifikasi OTP yang diizinkan
- **THEN** sistem menolak percobaan berikutnya untuk periode yang ditentukan dan tidak mengungkap OTP yang benar

#### Scenario: Melebihi batas permintaan OTP

- **WHEN** pengguna meminta OTP melebihi frekuensi yang diizinkan dalam jendela waktu
- **THEN** sistem menolak permintaan tambahan hingga jendela rate limit berlalu

#### Scenario: Race condition — OTP baru tidak terhapus oleh bump yang in-flight

- **WHEN** percobaan verifikasi gagal ke-5 memicu `bump_otp_attempts` (UPDATE attempts=5) dan pengguna secara bersamaan meminta OTP baru via `save_otp`
- **THEN** transaction pada `bump_otp_attempts` mencegah `save_otp` menulis di antara UPDATE dan DELETE, sehingga OTP baru tetap aman

#### Scenario: Redis rate limiter tidak permanent lockout

- **WHEN** Redis process crash terjadi setelah `INCR` tetapi sebelum `EXPIRE`
- **THEN** pada request berikutnya, `EXPIRE` tetap dipanggil (karena selalu dipanggil setiap request), sehingga key mendapatkan TTL yang benar
