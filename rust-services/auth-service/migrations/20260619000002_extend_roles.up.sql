-- Extend role RBAC dari 2 → 6 (multi-tier hierarchy)
-- Ref: openspec/changes/ws-multi-tier-rbac, D4
-- Hierarchy: super_admin(100) >= admin_iklan/admin_user(80) >= moderator(60) >= user_verified(40) >= user(20)

-- Hapus CHECK constraint lama
ALTER TABLE auth.users
    DROP CONSTRAINT IF EXISTS auth_users_role_check;

-- Tambah CHECK constraint baru dengan 6 nilai
ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_role_check
    CHECK (role IN ('super_admin', 'admin_iklan', 'admin_user', 'moderator', 'user_verified', 'user'));

-- Backfill: admin yang sudah ada → super_admin (transisi paling aman)
UPDATE auth.users
    SET role = 'super_admin'
    WHERE role = 'admin';
