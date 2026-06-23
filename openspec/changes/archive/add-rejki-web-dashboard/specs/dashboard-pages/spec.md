## ADDED Requirements

### Requirement: Halaman Iklan Pekerja dan Iklan Pekerjaan

Sistem klien SHALL menyediakan halaman untuk melihat daftar Iklan Pekerja dan Iklan Pekerjaan dengan kolom sesuai User Story, termasuk foto pekerjaan yang dapat dilihat pada pop-up. Admin SHALL dapat mencari, mengurutkan, men-suspend, dan mengekspor CSV.

#### Scenario: Admin melihat daftar iklan pekerja
- **WHEN** admin membuka halaman Iklan Pekerja
- **THEN** tabel menampilkan data dengan kolom ID, Nama Pekerja, Pengalaman Kerja, Upah, Jam Kerja, Cara Menghubungi, Foto Pekerjaan, dan Status

#### Scenario: Admin men-suspend iklan
- **WHEN** admin menceklis iklan dan menekan tombol Suspend dengan alasan dan bukti
- **THEN** iklan berubah status menjadi tersuspensi dan notifikasi terkirim ke pemilik

### Requirement: Halaman Iklan Pelatihan (tiga sub-halaman)

Sistem klien SHALL menyediakan tiga sub-halaman untuk Iklan Pelatihan: Daftar Pelatihan (dengan tombol Tambah, edit/cancel untuk admin, review untuk user), Konfirmasi Pelatihan (dengan review bukti transfer), dan Badge Pelatihan (dengan review sertifikat). Setiap sub-halaman SHALL menampilkan tombol kontekstual sesuai peran pembuat pelatihan atau status verifikasi.

#### Scenario: Admin membuat pelatihan (auto-approve)
- **WHEN** admin mengisi form Tambah Pelatihan dan menyimpan
- **THEN** pelatihan tercipta dengan status auto-approve

#### Scenario: Admin menolak pengajuan pelatihan pengguna
- **WHEN** admin menolak pelatihan milik pengguna dengan alasan
- **THEN** pelatihan berstatus ditolak dan notifikasi terkirim

### Requirement: Halaman Iklan Barang Bekas Gratis

Sistem klien SHALL menyediakan halaman daftar Iklan Barang Bekas Gratis dengan kolom sesuai User Story: ID, Judul, Deskripsi, Jenis Barang (Bekas/Baru), Foto Barang, Jumlah, Lokasi Pengambilan, Status. Admin SHALL dapat melihat pop-up foto, mencari, mengurutkan, men-suspend, dan mengekspor CSV.

#### Scenario: Admin melihat daftar barang bekas gratis
- **WHEN** admin membuka halaman Iklan Barang Bekas Gratis
- **THEN** tabel menampilkan data dengan kolom sesuai User Story termasuk foto barang yang dapat diklik untuk pop-up

### Requirement: Halaman Pengelolaan Pengguna

Sistem klien SHALL menyediakan halaman daftar pengguna baru yang mengajukan verifikasi data diri. Pop-up detail SHALL menampilkan data diri lengkap dengan NIK dan dokumen KTP/Selfie yang di-mask secara default dengan mekanisme click-to-view teraudit. Admin SHALL dapat menyetujui, menolak (dengan alasan), atau men-suspend pengguna.

#### Scenario: Admin mengklik untuk melihat NIK
- **WHEN** admin mengklik NIK yang ter-mask
- **THEN** NIK asli ditampilkan dan akses dicatat

#### Scenario: Admin menolak verifikasi dengan alasan
- **WHEN** admin menolak pengajuan verifikasi pengguna dengan alasan
- **THEN** pengguna menerima notifikasi penolakan

### Requirement: Halaman Pengelolaan Dukungan

Sistem klien SHALL menyediakan halaman daftar aduan dengan pop-up detail yang menampilkan keterangan dan foto bukti. Admin SHALL wajib mengisi tindakan sebelum menyetujui atau menolak aduan. Tombol Tolak dan Terima SHALL terkunci setelah aduan ditindaklanjuti.

#### Scenario: Admin menindaklanjuti aduan
- **WHEN** admin membuka pop-up aduan, mengisi tindakan, dan menekan Terima
- **THEN** aduan berstatus resolved dan notifikasi terkirim ke pelapor

#### Scenario: Admin menolak aduan tanpa tindakan
- **WHEN** admin mencoba menolak aduan tanpa mengisi tindakan
- **THEN** sistem klien menolak pengiriman dan meminta tindakan diisi

### Requirement: Halaman Corporate Communication

Sistem klien SHALL menyediakan halaman daftar artikel dengan kemampuan membuat artikel baru, menyunting, dan menghapus. Pembuat artikel SHALL ditampilkan sebagai read-only. Sistem klien SHALL mengirimkan permintaan ke backend yang akan menyiarkan notifikasi ke seluruh pengguna saat artikel diterbitkan atau diperbarui.

#### Scenario: Admin membuat artikel baru
- **WHEN** admin mengisi form artikel (judul, isi, kategori, foto) dan menyimpan
- **THEN** artikel terbit dan notifikasi broadcast dikirim ke seluruh pengguna

#### Scenario: Admin menyunting artikel
- **WHEN** admin menyunting artikel yang ada dan menyimpan
- **THEN** perubahan tersimpan, pembuat tetap read-only, dan notifikasi pembaruan dikirim
