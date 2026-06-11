## ADDED Requirements

### Requirement: Status akun mengikuti state machine yang ditentukan

Sistem SHALL menyimpan status akun pada entitas pengguna dengan salah satu nilai: `pending_verification`, `profile_incomplete`, `pending_kyc`, `rejected`, `active`, `suspended_temp`, `suspended_permanent`. Akun baru SHALL berstatus `pending_verification`. Sistem SHALL menjadi pemilik tunggal status akun (status TIDAK disimpan di domain lain).

#### Scenario: Akun baru berstatus awal
- **WHEN** sebuah akun berhasil diregistrasi
- **THEN** status akun bernilai `pending_verification`

#### Scenario: Verifikasi OTP register menaikkan status
- **WHEN** pengguna berhasil memverifikasi OTP dengan purpose `register`
- **THEN** status akun berubah dari `pending_verification` menjadi `profile_incomplete`

#### Scenario: Transisi tidak sah ditolak
- **WHEN** ada permintaan transisi status yang tidak diizinkan oleh state machine (mis. langsung dari `pending_verification` ke `active`)
- **THEN** sistem menolak transisi dan mempertahankan status saat ini

### Requirement: Transisi status hasil verifikasi KYC

Sistem SHALL menyediakan transisi `profile_incomplete → pending_kyc` ketika data diri lengkap diajukan, `pending_kyc → active` ketika disetujui, dan `pending_kyc → rejected` ketika ditolak. Akun `rejected` SHALL dapat mengajukan ulang menjadi `pending_kyc`. Transisi ini SHALL dipicu oleh domain lain melalui antarmuka client, bukan diputuskan di dalam domain pemicu.

#### Scenario: Pengajuan KYC memindahkan ke antrian verifikasi
- **WHEN** akun `profile_incomplete` mengirim seluruh data diri yang diwajibkan
- **THEN** status akun berubah menjadi `pending_kyc`

#### Scenario: KYC disetujui
- **WHEN** verifikasi KYC akun `pending_kyc` disetujui
- **THEN** status akun berubah menjadi `active`

#### Scenario: KYC ditolak lalu diajukan ulang
- **WHEN** verifikasi KYC akun `pending_kyc` ditolak, kemudian pengguna mengajukan ulang
- **THEN** status akun berubah menjadi `rejected`, lalu kembali menjadi `pending_kyc` saat pengajuan ulang diterima

### Requirement: Antarmuka AuthClient untuk membaca dan mengubah status akun

Sistem SHALL memperluas antarmuka `AuthClient` dengan operasi `get_account_status(user_id)` dan `set_account_status(user_id, status)`. Operasi ini SHALL dipanggil secara in-process oleh domain lain (mis. user-service, kapabilitas admin). Arah ketergantungan SHALL satu arah menuju auth-service tanpa membentuk siklus.

#### Scenario: Domain lain membaca status akun
- **WHEN** domain lain memanggil `get_account_status(user_id)` untuk user yang ada
- **THEN** sistem mengembalikan status akun terkini milik user tersebut

#### Scenario: Domain lain mengubah status akun melalui client
- **WHEN** kapabilitas KYC memanggil `set_account_status(user_id, active)` untuk transisi yang sah
- **THEN** sistem memperbarui status akun menjadi `active` dan perubahan tercermin pada pemeriksaan berikutnya

#### Scenario: Permintaan transisi tidak sah via client ditolak
- **WHEN** pemanggil meminta `set_account_status` dengan transisi yang melanggar state machine
- **THEN** sistem menolak permintaan tanpa mengubah status
