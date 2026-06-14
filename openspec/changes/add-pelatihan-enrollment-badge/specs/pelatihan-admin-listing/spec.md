## ADDED Requirements

### Requirement: Listing admin pelatihan dengan pencarian dan filter

Sistem SHALL menyediakan endpoint listing admin untuk ketiga sub-halaman pelatihan (Daftar Pelatihan, Konfirmasi Pelatihan, Badge Pelatihan) yang diproteksi otorisasi admin. Endpoint SHALL mendukung pencarian (judul/kode/penyelenggara), filter dan pengurutan berdasarkan status, serta paginasi sisi-server.

#### Scenario: Admin mencari pelatihan berdasarkan kata kunci
- **WHEN** admin meminta listing pelatihan dengan kata kunci pada judul, kode, atau penyelenggara
- **THEN** sistem mengembalikan pelatihan yang cocok terpaginasi

#### Scenario: Admin mengurutkan berdasarkan status
- **WHEN** admin meminta listing dengan filter/urutan berdasarkan status
- **THEN** sistem mengembalikan daftar terfilter/terurut sesuai

#### Scenario: Non-admin ditolak
- **WHEN** pemegang token non-admin mengakses listing admin pelatihan
- **THEN** sistem menolak dengan galat otorisasi

### Requirement: Ekspor CSV daftar pelatihan, konfirmasi, dan badge

Sistem SHALL menyediakan endpoint ekspor CSV untuk masing-masing dari ketiga sub-halaman yang menghasilkan unduhan sesuai filter yang sedang aktif.

#### Scenario: Admin mengekspor daftar pelatihan
- **WHEN** admin meminta ekspor CSV pada sub-halaman mana pun
- **THEN** sistem mengembalikan berkas CSV berisi header dan baris yang sesuai filter aktif
