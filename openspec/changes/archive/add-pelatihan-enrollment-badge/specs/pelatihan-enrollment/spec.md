## ADDED Requirements

### Requirement: Pendaftaran peserta pelatihan dengan bukti transfer

Sistem SHALL mengizinkan pengguna mendaftar pada pelatihan dan mengunggah bukti transfer. Pendaftaran SHALL berstatus `pending` hingga ditinjau admin.

#### Scenario: Pengguna mendaftar pelatihan
- **WHEN** pengguna mengirim permintaan pendaftaran pada pelatihan yang tersedia
- **THEN** sistem menciptakan enrollment berstatus `pending` dan menyediakan presigned URL untuk unggah bukti transfer

### Requirement: Admin meninjau dan memverifikasi pendaftaran

Sistem SHALL menyediakan operasi review pendaftaran bagi admin: menyetujui (status `approved`) atau menolak (status `rejected`) dengan alasan wajib bila menolak. Tombol Tolak dan Terima SHALL terkunci setelah verifikasi dilakukan.

#### Scenario: Admin menyetujui pendaftaran
- **WHEN** admin menyetujui enrollment `pending`
- **THEN** enrollment menjadi `approved` dan notifikasi dikirim ke pendaftar

#### Scenario: Admin menolak tanpa alasan
- **WHEN** admin menolak enrollment tanpa menyertakan alasan
- **THEN** sistem menolak permintaan dengan galat validasi

### Requirement: Notifikasi hasil verifikasi pendaftaran

Sistem SHALL mengirim notifikasi otomatis (email dan in-app) ke pendaftar setiap kali hasil verifikasi pendaftaran berubah.

#### Scenario: Pendaftar menerima notifikasi hasil
- **WHEN** admin menyetujui atau menolak pendaftaran
- **THEN** pendaftar menerima notifikasi email dan in-app
