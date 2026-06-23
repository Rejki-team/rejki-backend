## ADDED Requirements

### Requirement: Suspend banyak pengguna dalam satu permintaan

Sistem SHALL menyediakan endpoint admin yang men-suspend beberapa pengguna sekaligus dalam satu permintaan. Permintaan SHALL menyertakan daftar identitas pengguna, jenis suspend (sementara atau permanen), alasan yang wajib diisi, dan bukti pendukung. Untuk suspend sementara, waktu kedaluwarsa SHALL wajib disertakan. Endpoint SHALL diproteksi sehingga hanya pembawa token ber-role admin yang dapat mengaksesnya.

#### Scenario: Admin men-suspend beberapa pengguna sekaligus

- **WHEN** admin mengirim daftar beberapa pengguna dengan alasan dan bukti
- **THEN** sistem men-suspend setiap pengguna dan mengembalikan hasil per pengguna

#### Scenario: Alasan tidak diisi

- **WHEN** admin mengirim permintaan suspend tanpa alasan
- **THEN** sistem menolak permintaan dengan galat validasi sebelum memproses pengguna mana pun

#### Scenario: Suspend sementara tanpa waktu kedaluwarsa

- **WHEN** admin mengirim suspend sementara tanpa waktu kedaluwarsa
- **THEN** sistem menolak permintaan dengan galat validasi

### Requirement: Hasil per pengguna pada suspend massal

Sistem SHALL mengembalikan hasil terpisah untuk setiap pengguna dalam permintaan suspend massal, menandai keberhasilan atau kegagalan beserta alasan kegagalan. Kegagalan pada satu pengguna SHALL TIDAK membatalkan pemrosesan pengguna lain dalam permintaan yang sama.

#### Scenario: Sebagian pengguna gagal di-suspend

- **WHEN** admin men-suspend beberapa pengguna dan salah satunya tidak valid
- **THEN** sistem men-suspend pengguna yang valid dan menandai pengguna yang gagal beserta alasannya

### Requirement: Notifikasi otomatis ke pengguna ter-suspend

Sistem SHALL mengirim notifikasi otomatis melalui email dan dalam aplikasi kepada setiap pengguna yang berhasil di-suspend, memberitahukan tindakan suspend yang dilakukan.

#### Scenario: Pengguna menerima pemberitahuan suspend

- **WHEN** seorang pengguna berhasil di-suspend
- **THEN** pengguna tersebut menerima notifikasi email dan dalam aplikasi mengenai suspend
