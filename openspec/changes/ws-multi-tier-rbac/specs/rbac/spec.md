# multi-tier-rbac

## ADDED: Hierarchy rank-based role system

### Schema — Role hierarchy

Setiap role punya rank numerik. Otorisasi = `request.role.rank() >= endpoint.min_rank`.

| Role | Rank | DB Value |
|---|---|---|
| `super_admin` | 100 | `"super_admin"` |
| `admin_iklan` | 80 | `"admin_iklan"` |
| `admin_user` | 80 | `"admin_user"` |
| `moderator` | 60 | `"moderator"` |
| `user_verified` | 40 | `"user_verified"` |
| `user` | 20 | `"user"` (default) |

### SCENARIO: Admin endpoint accessible by super_admin

GIVEN user login dengan role `super_admin`
AND token valid dengan `role: "super_admin"`
WHEN request ke endpoint yang require rank >= 80
THEN middleware `require_role(80)` mengizinkan
AND request dilanjutkan ke handler

### SCENARIO: Admin endpoint blocked by user

GIVEN user login dengan role `user` (rank 20)
WHEN request ke endpoint yang require rank >= 80
THEN middleware `require_role(80)` menolak
AND response `403 FORBIDDEN` dengan error code `INSUFFICIENT_ROLE`

### SCENARIO: Moderator can access report endpoint

GIVEN user login dengan role `moderator` (rank 60)
WHEN request ke endpoint report yang require rank >= 60
THEN middleware `require_role(60)` mengizinkan

### SCENARIO: Moderator cannot access admin endpoint

GIVEN user login dengan role `moderator` (rank 60)
WHEN request ke endpoint admin yang require rank >= 80
THEN middleware `require_role(80)` menolak
AND response `403 FORBIDDEN` dengan error code `INSUFFICIENT_ROLE`

### SCENARIO: admin_login requires rank >= 80

GIVEN user dengan role `moderator` (rank 60) dan status active
WHEN user tersebut memanggil `AuthService::admin_login()`
THEN request ditolak sebagai `ServiceError::Unauthorized`

GIVEN user dengan role `admin_iklan` (rank 80) dan status active
WHEN user tersebut memanggil `AuthService::admin_login()`
THEN login sukses

### SCENARIO: Token lama tanpa role field

GIVEN token JWT lama yang tidak memiliki klaim `role`
WHEN token divalidasi
THEN `AuthClaims.role` = `None`
AND middleware `require_role(>= 40)` akan menolak (default-deny untuk fitur verified/user)
AND middleware `require_role(>= 20)` mengizinkan (user default)

### SCENARIO: Role diperluas — backfill admin lama

GIVEN database `auth.users` dengan role `'admin'` dari sebelum migration
WHEN migration baru dijalankan
THEN semua user dengan role `'admin'` berubah jadi `'super_admin'`
AND CHECK constraint diperluas ke 6 nilai baru

### SCENARIO: require_admin masih dipanggil (kode mati)

GIVEN compiler Rust
WHEN `cargo clippy --workspace -- -D warnings` dijalankan
THEN tidak ada warning tentang `require_admin` yang tidak terpakai
(Kode mati `require_admin()` sudah dihapus)
