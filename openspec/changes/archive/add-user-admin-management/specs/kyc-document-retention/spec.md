## ADDED Requirements

### Requirement: Pemusnahan otomatis dokumen saat penolakan KYC

Sistem SHALL menghapus dokumen KYC (Foto KTP dan Foto Selfie dengan KTP) secara otomatis ketika pengajuan verifikasi ditolak. Pemusnahan SHALL mencakup penghapusan objek dari penyimpanan dan pengosongan referensi dokumen pada pengajuan. Kegagalan penghapusan penyimpanan SHALL TIDAK membatalkan keputusan penolakan yang telah tercatat, dan SHALL dicatat untuk pembersihan lanjutan.

#### Scenario: Dokumen terhapus saat penolakan

- **WHEN** admin menolak pengajuan verifikasi KYC pengguna
- **THEN** dokumen KTP dan Selfie milik pengajuan tersebut dihapus secara otomatis dan referensinya dikosongkan

#### Scenario: Penghapusan penyimpanan gagal

- **WHEN** penolakan terjadi tetapi penghapusan objek dari penyimpanan gagal
- **THEN** keputusan penolakan tetap berlaku dan kegagalan penghapusan dicatat untuk pembersihan lanjutan

### Requirement: Penguncian pengajuan setelah verifikasi

Sistem SHALL mencegah peninjauan ulang terhadap pengajuan KYC yang telah berstatus terminal (disetujui atau ditolak). Permintaan peninjauan ulang SHALL ditolak untuk menjaga integritas audit dan mencegah pembukaan dokumen yang telah dimusnahkan.

#### Scenario: Peninjauan ulang ditolak

- **WHEN** admin mencoba meninjau pengajuan yang sudah disetujui atau ditolak
- **THEN** sistem menolak permintaan dan mempertahankan status terminal yang ada
