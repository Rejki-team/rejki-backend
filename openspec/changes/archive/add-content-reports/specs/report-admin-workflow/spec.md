## ADDED Requirements

### Requirement: Admin dapat meninjau daftar aduan

Sistem SHALL menyediakan endpoint listing aduan bagi admin (diproteksi otorisasi admin) dengan pencarian (ID Pengguna/ID Aduan), filter dan pengurutan berdasarkan status, serta paginasi. Admin SHALL dapat melihat detail aduan termasuk foto bukti.

#### Scenario: Admin mencari aduan berdasarkan ID
- **WHEN** admin mencari aduan dengan ID pengguna atau ID aduan
- **THEN** sistem mengembalikan aduan yang cocok terpaginasi

### Requirement: Admin menindaklanjuti aduan dengan tindakan wajib

Sistem SHALL menyediakan operasi tinjak lanjut aduan bagi admin. Admin SHALL menyetujui (status `resolved`) atau menolak (status `rejected`) aduan. Admin SHALL WAJIB mengisi `action_note` (tindakan yang telah dilakukan). Tombol Tolak dan Terima SHALL terkunci setelah aduan ditindaklanjuti.

#### Scenario: Admin menyelesaikan aduan dengan tindakan
- **WHEN** admin menyetujui aduan dengan `action_note` yang menjelaskan tindakan yang diambil
- **THEN** aduan menjadi `resolved` dan tindakan tersimpan

#### Scenario: Admin menolak aduan
- **WHEN** admin menolak aduan dengan `action_note`
- **THEN** aduan menjadi `rejected` dan alasan tersimpan

#### Scenario: Admin tidak dapat menindaklanjuti aduan yang sudah selesai
- **WHEN** admin mencoba menyetujui atau menolak aduan yang sudah `resolved` atau `rejected`
- **THEN** sistem menolak operasi tersebut

### Requirement: Notifikasi hasil tindak lanjut

Sistem SHALL mengirim notifikasi otomatis (email dan in-app) ke pelapor setiap kali aduan ditindaklanjuti (disetujui atau ditolak).

#### Scenario: Pelapor menerima notifikasi hasil
- **WHEN** admin menyetujui atau menolak aduan
- **THEN** pelapor menerima notifikasi email dan in-app

### Requirement: Ekspor CSV daftar aduan

Sistem SHALL menyediakan endpoint ekspor CSV daftar aduan (diproteksi otorisasi admin) yang mengikuti filter aktif.

#### Scenario: Admin mengekspor daftar aduan
- **WHEN** admin meminta ekspor CSV
- **THEN** sistem mengembalikan berkas CSV sesuai filter
