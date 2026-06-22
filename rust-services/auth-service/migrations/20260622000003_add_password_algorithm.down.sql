-- W3D-14 (D5): Rollback password_algorithm column

DROP INDEX IF EXISTS auth.idx_users_password_algorithm;
ALTER TABLE auth.users DROP COLUMN IF EXISTS password_algorithm;
