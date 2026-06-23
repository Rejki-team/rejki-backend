## ADDED Requirements

### Requirement: Pengajuan sertifikat pelatihan oleh peserta

Sistem SHALL mengizinkan peserta pelatihan mengajukan sertifikat untuk diverifikasi admin. Pengajuan SHALL berstatus `pending` hingga ditinjau.

#### Scenario: Peserta mengajukan sertifikat
- **WHEN** peserta mengirim permintaan pengajuan badge dengan sertifikat
- **THEN** sistem menciptakan badge berstatus `pending` dan menyediakan presigned URL untuk unggah sertifikat

### Requirement: Admin meninjau sertifikat

Sistem SHALL menyediakan operasi review badge bagi admin: menyetujui (status `approved`, `approved_at` tercatat) atau menolak (status `rejected`) dengan alasan wajib. Tombol Tolak dan Terima SHALL terkunci setelah verifikasi.

#### Scenario: Admin menyetujui sertifikat
- **WHEN** admin menyetujui badge `pending`
- **THEN** badge menjadi `approved` dan `approved_at` mencatat waktu persetujuan

#### Scenario: Admin menolak sertifikat
- **WHEN** admin menolak badge dengan alasan
- **THEN** badge menjadi `rejected` dan alasan tersimpan

### Requirement: Notifikasi hasil verifikasi badge

Sistem SHALL mengirim notifikasi otomatis (email dan in-app) ke pengaju setiap kali hasil verifikasi badge berubah.

#### Scenario: Pengaju menerima notifikasi hasil
- **WHEN** admin menyetujui atau menolak badge
- **THEN** pengaju menerima notifikasi email dan in-app
