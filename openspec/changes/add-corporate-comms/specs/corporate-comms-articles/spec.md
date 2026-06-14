## ADDED Requirements

### Requirement: Admin dapat mengelola artikel corporate communication

Sistem SHALL menyediakan operasi CRUD artikel bagi admin (diproteksi otorisasi admin): melihat daftar, membuat, melihat detail, menyunting, dan menghapus (soft-delete). Pembuat artikel SHALL dicatat dari identitas admin yang sedang aktif dan bersifat read-only (tidak dapat diubah). Kategori artikel SHALL bersifat dropdown dengan nilai awal `informasi` dan dirancang extensible.

#### Scenario: Admin membuat artikel
- **WHEN** admin membuat artikel dengan judul, isi, dan kategori
- **THEN** artikel tercipta dengan `author_id` sesuai sesi admin dan dapat disertai foto

#### Scenario: Admin menyunting artikel
- **WHEN** admin menyunting artikel yang ada
- **THEN** perubahan tersimpan, `author_id` tetap tidak berubah, dan `updated_at` diperbarui

#### Scenario: Admin menghapus artikel (soft-delete)
- **WHEN** admin menghapus artikel
- **THEN** artikel ditandai `deleted_at` dan tidak muncul di daftar

#### Scenario: Non-admin ditolak
- **WHEN** pengguna non-admin mencoba mengakses endpoint artikel
- **THEN** sistem menolak dengan galat otorisasi

### Requirement: Daftar artikel dapat dicari dan diurutkan

Sistem SHALL mendukung pencarian artikel berdasarkan judul dan pengurutan berdasarkan kategori pada endpoint listing admin.

#### Scenario: Admin mencari artikel berdasarkan judul
- **WHEN** admin mencari dengan kata kunci judul
- **THEN** sistem mengembalikan artikel yang judulnya cocok, terpaginasi
