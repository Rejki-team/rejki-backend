# patch-iklan

## ADDED: PATCH endpoint untuk 4 iklan service

### SCENARIO: Update sebagian field

GIVEN user login dengan akun aktif
AND user memiliki iklan dengan id `X` (moderation_status = active)
WHEN user mengirim PATCH /{id} dengan body `{ "judul": "baru" }`
THEN hanya field `judul` yang berubah
AND response 200 dengan data iklan setelah update
AND `updated_at` ter-set ke waktu sekarang

### SCENARIO: IDOR — update iklan milik orang lain

GIVEN user A login
WHEN user A mengirim PATCH /{id} dimana iklan {id} milik user B
THEN response 404 Not Found
AND tidak ada data yang bocor tentang kepemilikan iklan

### SCENARIO: Update iklan yang di-suspend

GIVEN iklan dalam status SuspendedTemp atau SuspendedPermanent
WHEN user pemilik mengirim PATCH /{id}
THEN response 403 Forbidden dengan pesan "iklan sedang ditangguhkan"

### SCENARIO: Update pelatihan yang sudah berjalan

GIVEN iklan pelatihan dengan status `PelatihanBerjalan`
WHEN user pemilik mengirim PATCH /{id}
THEN response 403 Forbidden dengan pesan "tidak dapat mengubah iklan yang sudah berjalan atau selesai"

### SCENARIO: Partial update dengan semua field None

GIVEN PATCH /{id} dengan body `{}`
WHEN request diproses
THEN response 200 dengan data iklan yang tidak berubah
AND updated_at tetap update (karena query tetap jalan)

### SCENARIO: Update dengan validasi gagal

GIVEN PATCH /{id} dengan body `{ "judul": "ab" }`
WHEN validasi length(min=3) gagal
THEN response 422 Unprocessable Entity

### SCENARIO: Immutable field diabaikan

GIVEN PATCH /{id} dengan body `{ "moderation_status": "active" }`
WHEN request diproses
THEN field moderation_status tidak berubah (tidak ada di Update*Input DTO)
