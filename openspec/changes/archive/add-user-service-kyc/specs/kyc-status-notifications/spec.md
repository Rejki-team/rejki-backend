## ADDED Requirements

### Requirement: Status verifikasi dapat dilihat kapan saja

Sistem SHALL membuat status verifikasi KYC tersedia bagi pengguna kapan saja melalui profil (`GET /api/v1/users/me`), termasuk alasan penolakan bila berstatus ditolak.

#### Scenario: Pengguna memeriksa status verifikasi
- **WHEN** pengguna memanggil `GET /api/v1/users/me`
- **THEN** sistem mengembalikan status KYC terkini (mis. `pending`, `approved`, `rejected`), dan alasan bila ditolak

### Requirement: Notifikasi pada setiap perubahan status verifikasi

Sistem SHALL mengirim notifikasi setiap kali status verifikasi berubah, melalui notification-service. Kanal push dan in-app SHALL dipakai untuk setiap perubahan status. Kanal email SHALL dipakai hanya pada peristiwa awal (submission dibuat) dan hasil akhir (approved atau rejected).

#### Scenario: Notifikasi saat submission dibuat (awal)
- **WHEN** submission KYC dibuat
- **THEN** sistem mengirim notifikasi push, in-app, dan email yang memberitahu bahwa verifikasi sedang diproses

#### Scenario: Notifikasi pada hasil akhir
- **WHEN** submission disetujui atau ditolak
- **THEN** sistem mengirim notifikasi push, in-app, dan email berisi hasil; bila ditolak, menyertakan alasan dan ajakan memperbaiki (menghormati cooldown 3 hari kerja)

#### Scenario: Perubahan status antara tanpa email
- **WHEN** status berubah pada peristiwa antara yang bukan awal/hasil akhir (mis. mulai ditinjau)
- **THEN** sistem mengirim notifikasi push dan in-app saja, tanpa email

### Requirement: Notifikasi dicatat di daftar in-app

Sistem SHALL memastikan pemberitahuan status verifikasi tercatat pada daftar notifikasi in-app pengguna sehingga dapat dilihat kembali.

#### Scenario: Pemberitahuan tersedia di daftar in-app
- **WHEN** sebuah notifikasi status verifikasi dikirim
- **THEN** pemberitahuan tersebut muncul pada daftar notifikasi in-app pengguna
