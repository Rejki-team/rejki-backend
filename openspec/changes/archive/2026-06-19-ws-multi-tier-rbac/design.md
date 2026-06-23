## Context

Saat ini role hanya `User` dan `Admin` dengan cek binary `is_admin()`. Middleware `require_admin()` ada di `common/auth-middleware/src/lib.rs`. Role enum di `auth-service-client/src/lib.rs`. CHECK constraint di migration `20260614000001_add_role_to_users.up.sql`: `CHECK (role IN ('user', 'admin'))`.

Design ini memilih **pure hierarchy model** (rank-based) sesuai keputusan user — role yang lebih tinggi bisa akses semua endpoint yang role lebih rendah bisa akses.

## Hierarchy

| Role | Rank | Akses |
|---|---|---|
| `super_admin` | 100 | Semua endpoint (ALL) |
| `admin_iklan` | 80 | Iklan admin + user admin + moderator + user |
| `admin_user` | 80 | User/KYC admin + moderator + user |
| `moderator` | 60 | Report + corporate-comms + user |
| `user_verified` | 40 | Post iklan, chat, create reports |
| `user` | 20 | Browse, register, login (default) |

## Decisions

### D1: Hierarchy rank-based (pure hierarchy), bukan module-based atau permission-based

**Pilihan**: Setiap role punya `rank() -> u8`. Guard = `rank >= min_rank`.

**Alternatif A (module-based)**: Setiap admin scope punya middleware sendiri (`require_admin_iklan`, `require_admin_user`). Lebih granular tapi kompleksitas routing tinggi.

**Alternatif B (permission-based)**: Fully granular — setiap endpoint punya izin spesifik. Overkill untuk ukuran project saat ini.

**Rasional**: User memilih pure hierarchy. Trade-off: `admin_iklan` dan `admin_user` rank sama (80) — bisa akses endpoint satu sama lain. Ini acceptable karena keduanya adalah admin terpercaya, dan alternatif module-based akan menambah ~5x lebih banyak middleware.

### D2: `require_role(min_rank)` factory function, bukan generic middleware

**Pilihan**: Fungsi `pub fn require_role(min_rank: u8)` mengembalikan `impl Fn(Request, Next) -> Result<Response, AppError>`, dipasang via `axum::middleware::from_fn`.

**Alternatif**: Middleware struct dengan field `min_rank` — lebih OOP, tapi butuh lapisan dari_fn tambahan.

**Rasional**: Fungsi factory paling idiomatis Axum. Pattern sama dengan pola `from_fn` yang sudah ada. Tidak perlu trait baru.

### D3: `AppError::InsufficientRole` (403) berbeda dari `AccountNotAdmin` yang dihapus

**Pilihan**: Varian error baru. `AccountNotAdmin` TETAP dipertahankan di enum (backward compat) tapi jadi unreachable setelah semua caller diganti.

**Rasional**: Kode error spesifik (`INSUFFICIENT_ROLE`) memberi informasi lebih jelas ke client daripada `ACCOUNT_NOT_ADMIN` yang ambigu setelah ada 6 role. `AccountNotAdmin` dipertahankan untuk menghindari breaking change API response code.

### D4: Backfill admin yang sudah ada → `super_admin`

**Pilihan**: Migration mengubah semua `role = 'admin'` yang ada menjadi `'super_admin'`.

**Rasional**: Admin yang sudah ada sebelum migration ini adalah satu-satunya admin. Memberi mereka `super_admin` adalah transisi paling aman — akses mereka tidak berkurang. Admin baru setelah migration bisa di-set ke role yang sesuai.

### D5: `admin_login` guard minimum rank 80

**Pilihan**: AuthService `admin_login()` cek `user.role.rank() >= 80` (admin_iklan atau super_admin), bukan `rank() >= 60` (moderator).

**Rasional**: Moderator adalah role untuk report/comms moderation — tidak seharusnya bisa login ke endpoint admin (suspend user, KYC, manage iklan). Memberi moderator akses admin login adalah over-privilege.

## Risks / Trade-offs

- **Risk**: admin_iklan dan admin_user rank sama (80) → cross-domain access → **Mitigasi**: Acceptable. Keduanya admin terpercaya. Bedanya lebih ke organisasi/bisnis daripada keamanan teknis.
- **Risk**: Backward compat — token lama tanpa field role → **Mitigasi**: `claims.role` tetap `Option<Role>`, None → default ke `Role::User` (rank 20). Token lama tidak bisa akses endpoint yang require rank >= 40.
- **Risk**: Migration alter CHECK → **Mitigasi**: ALTER TABLE dengan IF ada pattern, value baru aditif. Apakah ada role yang perlu dihapus? Tidak — `admin` diganti jadi `super_admin`.
