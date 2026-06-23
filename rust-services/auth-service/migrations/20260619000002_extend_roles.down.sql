-- Rollback: kembali ke 2 role (user/admin)
-- super_admin → admin, lainnya → user (safe default)

-- Kembalikan super_admin ke admin
UPDATE auth.users
    SET role = 'admin'
    WHERE role = 'super_admin';

-- Role baru lainnya tidak ada padanan di schema lama → fallback ke 'user'
UPDATE auth.users
    SET role = 'user'
    WHERE role NOT IN ('user', 'admin');

-- Hapus CHECK constraint baru
ALTER TABLE auth.users
    DROP CONSTRAINT IF EXISTS auth_users_role_check;

-- Kembalikan CHECK constraint lama
ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_role_check
    CHECK (role IN ('user', 'admin'));
