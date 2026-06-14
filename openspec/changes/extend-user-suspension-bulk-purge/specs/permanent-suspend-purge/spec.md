## ADDED Requirements

### Requirement: Pemusnahan otomatis dokumen saat suspend permanen

Sistem SHALL menghapus dokumen KYC (Foto KTP dan Foto Selfie dengan KTP) secara otomatis untuk setiap pengguna yang di-suspend secara permanen. Pemusnahan SHALL dilakukan setelah suspend tercatat. Kegagalan pemusnahan SHALL TIDAK membatalkan suspend yang telah tercatat, dan SHALL dicatat untuk pembersihan lanjutan.

#### Scenario: Dokumen terhapus saat suspend permanen

- **WHEN** admin men-suspend seorang pengguna secara permanen
- **THEN** dokumen KTP dan Selfie pengguna tersebut dihapus secara otomatis

#### Scenario: Suspend sementara tidak menghapus dokumen

- **WHEN** admin men-suspend seorang pengguna secara sementara
- **THEN** dokumen pengguna tersebut tetap dipertahankan

#### Scenario: Pemusnahan gagal saat suspend permanen

- **WHEN** suspend permanen tercatat tetapi pemusnahan dokumen gagal
- **THEN** suspend tetap berlaku dan kegagalan pemusnahan dicatat untuk pembersihan lanjutan
