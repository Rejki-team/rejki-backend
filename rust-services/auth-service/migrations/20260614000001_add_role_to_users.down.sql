-- Rollback: hapus kolom role dari auth.users
ALTER TABLE auth.users DROP CONSTRAINT IF EXISTS auth_users_role_check;
ALTER TABLE auth.users DROP COLUMN IF EXISTS role;
