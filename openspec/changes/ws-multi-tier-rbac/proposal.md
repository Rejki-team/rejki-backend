## Mengapa

Saat ini hanya ada 2 role (`User` dan `Admin`) dengan middleware binary `require_admin()`. Phase 3 mensyaratkan multi-tier RBAC (W3B-05) untuk:

1. **Pemisahan tugas admin** — admin iklan tidak perlu akses ke endpoint manajemen user, moderator tidak perlu akses suspend akun. Dengan satu role `Admin`, semua admin punya akses penuh ke semua endpoint admin — terlalu besar.
2. **Skalabilitas moderasi** — butuh role `moderator` untuk menangani report/konten tanpa akses ke data sensitif (KYC, suspend permanen).
3. **Tier user** — user yang sudah terverifikasi (verified) bisa dibedakan dari user baru (biasa) untuk gating fitur premium (post iklan, chat).

## What Changes

- **Role enum diperluas** — dari 2 (`User`, `Admin`) menjadi 6 (`super_admin`, `admin_iklan`, `admin_user`, `moderator`, `user_verified`, `user`) dengan `rank()` method untuk hierarchy comparison.
- **Migration alter CHECK constraint** — `auth.users.role` CHECK diubah jadi 6 nilai baru.
- **Middleware baru** — `require_role(min_rank: u8)` menggantikan `require_admin()`. Generic factory function yang bisa dikonfigurasi per service router.
- **AppError baru** — `InsufficientRole` untuk response ketika role tidak mencukupi (403).
- **Router update** — 8 service interface files mengganti `.layer(from_fn(require_admin))` dengan `.layer(from_fn(require_role(min_rank)))`.
- **AuthService guard** — `admin_login()` memperketat guard jadi rank >= 80 (admin_iklan ke atas, bukan moderator).

## New Capabilities

- `multi-tier-rbac`: 6 role RBAC hierarchy dengan rank-based authorization, middleware `require_role()` yang bisa dikonfigurasi per endpoint/service.

## Modified Capabilities

- `auth-service`: `admin_login()` guard dari `is_admin()` → `rank() >= MIN_ADMIN_RANK`
- `auth-middleware`: `require_admin()` → `require_role(min_rank: u8)`
- `errors`: tambah `AppError::InsufficientRole` (403 FORBIDDEN, INSUFFICIENT_ROLE)

## Impact

- **auth-service-client**: Role enum + rank() method — tambah 4 variant baru, tidak ada variant yang dihapus
- **auth-service**: migration alter CHECK constraint + admin_login guard
- **common/auth-middleware**: `require_role()` factory function baru
- **common/errors**: tambah 1 varian `AppError`
- **8 service interface files**: ganti middleware dari `require_admin` ke `require_role`
- **Integration test fixtures**: update role value di fixture
- **Swagger**: update Role enum doc di openapi.rs
- **Tidak ada endpoint baru**, tidak ada perubahan protokol API
