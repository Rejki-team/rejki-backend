## ADDED Requirements

### Requirement: Pengguna melihat profil sendiri beserta status KYC

Sistem SHALL mengizinkan pengguna terautentikasi melihat profil miliknya, mencakup field profil dan status verifikasi KYC saat ini. Data sensitif NIK SHALL ditampilkan dalam bentuk ter-mask (mis. hanya 4 digit terakhir), bukan nilai penuh.

#### Scenario: Melihat profil sendiri
- **WHEN** pengguna terautentikasi memanggil `GET /api/v1/users/me`
- **THEN** sistem mengembalikan profil miliknya beserta status KYC, dengan NIK ditampilkan ter-mask

#### Scenario: NIK tidak pernah ditampilkan penuh
- **WHEN** profil yang memuat NIK dikembalikan ke klien
- **THEN** nilai NIK penuh tidak disertakan; hanya bentuk ter-mask yang dikembalikan

### Requirement: Pengguna memperbarui field profil non-KYC

Sistem SHALL mengizinkan pengguna memperbarui field profil non-KYC (mis. `full_name`, `bio`). Sistem SHALL menolak perubahan field KYC terverifikasi melalui endpoint pembaruan profil biasa. NIK SHALL tidak dapat diubah melalui endpoint ini.

#### Scenario: Memperbarui field profil yang diizinkan
- **WHEN** pengguna memanggil `PATCH /api/v1/users/me` dengan field profil non-KYC yang valid
- **THEN** sistem menyimpan perubahan dan mengembalikan profil terbaru

#### Scenario: Upaya mengubah NIK via update profil ditolak
- **WHEN** pengguna menyertakan perubahan NIK pada `PATCH /api/v1/users/me`
- **THEN** sistem menolak perubahan NIK (NIK immutable) tanpa mengubah nilai tersimpan

### Requirement: Profil hanya dapat diakses pemiliknya

Sistem SHALL memastikan operasi lihat/ubah profil sendiri hanya berlaku untuk profil milik pemanggil. Akses ke data milik orang lain melalui endpoint "me" SHALL tidak dimungkinkan.

#### Scenario: Ownership ditegakkan
- **WHEN** pengguna mengakses `GET`/`PATCH /api/v1/users/me`
- **THEN** sistem hanya membaca/mengubah profil yang terkait dengan identitas pada token, bukan profil pengguna lain
