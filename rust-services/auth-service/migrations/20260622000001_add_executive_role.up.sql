-- Tambah role Executive (rank 90) untuk akses insights/analytics read-only.
-- Executive TIDAK bisa mengakses admin endpoints (hanya SuperAdmin/AdminIklan/AdminUser).
-- Ref: openspec/changes/ws-ceo-analytics, D1
-- Hierarchy: super_admin(100) >= executive(90) >= admin_iklan/admin_user(80) >= moderator(60) >= user_verified(40) >= user(20)

-- Hapus CHECK constraint lama
ALTER TABLE auth.users
    DROP CONSTRAINT IF EXISTS auth_users_role_check;

-- Tambah CHECK constraint baru dengan 7 nilai (+ executive)
ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_role_check
    CHECK (role IN ('super_admin', 'executive', 'admin_iklan', 'admin_user', 'moderator', 'user_verified', 'user'));
