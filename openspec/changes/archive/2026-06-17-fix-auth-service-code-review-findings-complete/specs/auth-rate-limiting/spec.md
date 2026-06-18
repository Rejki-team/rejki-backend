## ADDED Requirements

### Requirement: Rate limiting pada endpoint login berbasis email

Sistem SHALL membatasi frekuensi percobaan login (`/login` dan `/admin/login`) per alamat email dalam jendela waktu tertentu. Rate limit SHALL dicek SEBELUM bcrypt password verification untuk menghindari pemborosan CPU pada serangan brute-force. Rate limit SHALL menggunakan Redis counter dengan key `login_req:{endpoint}:{email_hash}` (email di-hash SHA-256 untuk privasi).

#### Scenario: Percobaan login dalam batas rate limit

- **WHEN** pengguna mengirim maksimal 5 percobaan login ke `/login` dengan email yang sama dalam 15 menit
- **THEN** setiap percobaan diverifikasi normal (bcrypt password check berjalan)

#### Scenario: Percobaan login melebihi rate limit

- **WHEN** pengguna mengirim lebih dari 5 percobaan login ke `/login` dengan email yang sama dalam 15 menit
- **THEN** sistem menolak dengan HTTP 401 `UNAUTHORIZED` tanpa pesan spesifik, tanpa menjalankan bcrypt password verification

#### Scenario: Rate limit pada `/admin/login`

- **WHEN** pengguna mengirim lebih dari 5 percobaan login ke `/admin/login` dengan email yang sama dalam 15 menit
- **THEN** sistem menolak dengan HTTP 401 `UNAUTHORIZED` tanpa pesan spesifik, tanpa menjalankan bcrypt password verification

#### Scenario: Counter rate limit per endpoint independen

- **WHEN** pengguna yang sama di-rate-limit di `/login`
- **THEN** pengguna tetap dapat mengakses `/admin/login` (counter terpisah per endpoint)

#### Scenario: Fail-open saat Redis tidak tersedia

- **WHEN** Redis connection gagal saat pengecekan rate limit
- **THEN** sistem tetap mengizinkan login (fail-open) dengan log peringatan

### Requirement: Konfigurasi rate limit login dapat disetel via environment variable

Sistem SHALL membaca konfigurasi rate limit dari environment variable `LOGIN_RATE_LIMIT_MAX` (default 5) dan `LOGIN_RATE_LIMIT_WINDOW_SECS` (default 900). Nilai-nilai ini SHALL dikonversi ke named constants internal untuk menghindari hardcoded.

#### Scenario: Konfigurasi default

- **WHEN** environment variable `LOGIN_RATE_LIMIT_MAX` dan `LOGIN_RATE_LIMIT_WINDOW_SECS` tidak diset
- **THEN** sistem menggunakan default 5 percobaan dalam 15 menit

#### Scenario: Konfigurasi kustom

- **WHEN** `LOGIN_RATE_LIMIT_MAX=10` dan `LOGIN_RATE_LIMIT_WINDOW_SECS=600` diset di environment
- **THEN** sistem menggunakan 10 percobaan dalam 10 menit
