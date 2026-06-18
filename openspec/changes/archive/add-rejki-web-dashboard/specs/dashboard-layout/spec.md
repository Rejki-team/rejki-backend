## ADDED Requirements

### Requirement: Sidebar navigasi collapsible

Sistem klien SHALL menyediakan sidebar yang memuat daftar menu vertikal beserta deskripsinya sesuai User Story. Sidebar SHALL dapat dibuka dan ditutup melalui ikon hamburger. Sidebar SHALL responsif — overlay pada layar kecil, sticky pada layar besar.

#### Scenario: Admin membuka dan menutup sidebar
- **WHEN** admin menekan ikon hamburger
- **THEN** sidebar membuka (jika tertutup) atau menutup (jika terbuka)

#### Scenario: Menu dan sub-menu sesuai User Story
- **WHEN** admin melihat sidebar
- **THEN** menu Iklan Pelatihan menampilkan tiga sub-menu: Daftar Pelatihan, Konfirmasi Pelatihan, Badge Pelatihan

### Requirement: Navigasi antar halaman via router

Sistem klien SHALL menggunakan router klien untuk menavigasi antar halaman tanpa pemuatan ulang penuh. Setiap menu dan sub-menu SHALL tertaut ke rute yang sesuai.

#### Scenario: Admin mengklik menu
- **WHEN** admin mengklik item menu pada sidebar
- **THEN** konten halaman yang sesuai dimuat tanpa pemuatan ulang halaman penuh
