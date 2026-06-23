## ADDED Requirements

### Requirement: Broadcast notifikasi saat artikel diterbitkan atau diperbarui

Sistem SHALL menyiarkan notifikasi in-app ke seluruh pengguna aktif setiap kali artikel baru diterbitkan atau artikel yang sudah ada diperbarui. Notifikasi SHALL bersifat fire-and-forget (kegagalan pengiriman tidak menggagalkan pembuatan atau pembaruan artikel).

#### Scenario: Notifikasi saat artikel baru terbit
- **WHEN** admin menerbitkan artikel baru
- **THEN** seluruh pengguna aktif menerima notifikasi in-app tentang artikel baru tersebut

#### Scenario: Notifikasi saat artikel diperbarui
- **WHEN** admin memperbarui artikel yang sudah ada
- **THEN** seluruh pengguna aktif menerima notifikasi in-app tentang pembaruan artikel

#### Scenario: Tidak ada notifikasi saat artikel dihapus
- **WHEN** admin menghapus artikel
- **THEN** sistem tidak mengirim notifikasi broadcast

### Requirement: Notifikasi tidak memuat isi penuh artikel

Sistem SHALL mengirimkan judul artikel dan indikasi "baru" atau "diperbarui" pada payload notifikasi. Isi penuh artikel SHALL TIDAK disertakan dalam notifikasi untuk menjaga payload tetap ringkas.

#### Scenario: Payload notifikasi ringkas
- **WHEN** sistem menyiarkan notifikasi artikel
- **THEN** payload hanya memuat judul dan jenis pembaruan (baru/diperbarui), tanpa isi penuh artikel
