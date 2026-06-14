## ADDED Requirements

### Requirement: Listing admin iklan dengan pencarian, filter, urutan, dan paginasi

Sistem SHALL menyediakan endpoint listing admin per vertikal (diproteksi otorisasi admin) yang mengembalikan kolom sesuai kebutuhan moderasi termasuk status. Endpoint SHALL mendukung pencarian (judul/kode/pembuat), filter dan pengurutan berdasarkan status moderasi, dan paginasi sisi-server.

#### Scenario: Admin mencari iklan berdasarkan kata kunci
- **WHEN** admin meminta listing dengan kata kunci pencarian
- **THEN** sistem mengembalikan hanya iklan yang cocok pada judul, kode, atau pembuat, terpaginasi

#### Scenario: Admin mengurutkan berdasarkan status
- **WHEN** admin meminta listing dengan filter/urutan berdasarkan status moderasi
- **THEN** sistem mengembalikan iklan terfilter/terurut sesuai status

#### Scenario: Non-admin ditolak
- **WHEN** pemegang token non-admin mengakses listing admin
- **THEN** sistem menolak dengan galat otorisasi

### Requirement: Admin tidak dapat menyunting data iklan

Sistem SHALL TIDAK menyediakan operasi penyuntingan data iklan bagi admin. Satu-satunya operasi mutasi admin atas iklan SHALL berupa suspend (dan soft-delete terkait moderasi).

#### Scenario: Tidak ada endpoint edit iklan untuk admin
- **WHEN** admin mencoba mengubah konten data iklan (judul, deskripsi, dsb.)
- **THEN** sistem tidak menyediakan jalur tersebut (admin bersifat read-only kecuali suspend)
