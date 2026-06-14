-- Tambah kolom role pada auth.users (RBAC admin, extensible)
-- Ref: openspec/changes/add-admin-rbac

ALTER TABLE auth.users
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user';

-- CHECK constraint extensible: tambah nilai baru di masa depan aditif tanpa ALTER TYPE.
ALTER TABLE auth.users
    ADD CONSTRAINT auth_users_role_check
    CHECK (role IN ('user', 'admin'));

-- Backfill: semua baris existing otomatis 'user' via DEFAULT.
