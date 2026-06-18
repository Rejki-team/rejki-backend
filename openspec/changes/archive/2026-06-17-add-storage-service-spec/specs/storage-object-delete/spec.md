## ADDED Requirements

### Requirement: Penghapusan objek dari storage

Sistem SHALL menyediakan operasi hapus objek melalui `StorageClient::delete(object_key)`. Operasi ini dipakai untuk pemusnahan dokumen sesuai kebijakan retensi (mis. K11 UU PDP — dokumen KYC dimusnahkan setelah akun ditutup + periode dispute).

#### Scenario: Hapus objek yang ada
- **WHEN** klien terotorisasi menghapus object key yang ada di storage
- **THEN** sistem menghapus objek dan mengembalikan `Ok(())`

#### Scenario: Hapus objek yang tidak ada
- **WHEN** klien menghapus object key yang tidak ada
- **THEN** operasi tetap berhasil (`Ok(())`) — idempoten

#### Scenario: Storage tidak tersedia
- **WHEN** MinIO endpoint tidak dikonfigurasi
- **THEN** sistem menolak dengan `StorageClientError::Unavailable`

### Requirement: Pemusnahan dokumen tidak dapat dibatalkan

Sistem SHALL TIDAK menyediakan mekanisme undo/restore setelah penghapusan. Application layer SHALL memastikan konfirmasi dan audit trail sebelum memanggil `delete`.

#### Scenario: Objek yang sudah dihapus tidak dapat diakses
- **WHEN** klien meminta presigned download untuk object key yang sudah dihapus
- **THEN** akses ke URL presigned akan gagal (objek tidak ditemukan di MinIO)
