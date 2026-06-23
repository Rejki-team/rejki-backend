-- W3D-14 (D5): Tambah column password_algorithm untuk dual-support bcrypt/argon2
-- Default 'bcrypt' untuk backward compatibility

ALTER TABLE auth.users ADD COLUMN password_algorithm TEXT NOT NULL DEFAULT 'bcrypt';

CREATE INDEX idx_users_password_algorithm ON auth.users (password_algorithm);
