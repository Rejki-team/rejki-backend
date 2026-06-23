-- Rollback: kembali ke 6 roles (tanpa executive)
-- executive → fallback ke user (safe default — non-admin)

UPDATE auth.users
    SET role = 'user'
    WHERE role = 'executive';

ALTER TABLE auth.users
    DROP CONSTRAINT IF EXISTS auth_users_role_check;

ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_role_check
    CHECK (role IN ('super_admin', 'admin_iklan', 'admin_user', 'moderator', 'user_verified', 'user'));
