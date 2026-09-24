DROP INDEX IF EXISTS auth.uq_auth_users_phone_hash;
ALTER TABLE auth.users DROP COLUMN IF EXISTS phone_hash;
