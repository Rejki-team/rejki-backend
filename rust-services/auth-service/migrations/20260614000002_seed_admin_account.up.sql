-- Seed akun admin (idempoten — gunakan ON CONFLICT DO NOTHING).
-- Password hash harus di-generate DBA menggunakan bcrypt (cost 12) sebelum menjalankan migrasi ini.
-- Ganti placeholder di bawah dengan hash sebenarnya.
--
-- Cara generate hash (via CLI):
--   Menggunakan Python: python -c "import bcrypt; print(bcrypt.hashpw(b'<password>', bcrypt.gensalt(12)).decode())"
--   Atau jalankan: cargo run --bin seed-admin -- --email admin@rejki.id --password <password>
--
-- Setelah seed, DBA wajib merotasi kredensial via prosedur operasional.
-- Ref: openspec/changes/add-admin-rbac D5.

-- Hanya insert bila belum ada (idempoten).
INSERT INTO auth.users (id, email, password_hash, status, role, tos_accepted_at, tos_version)
VALUES (
    gen_random_uuid(),
    'admin@rejki.id',
    -- REPLACE_ME: ganti dengan bcrypt hash dari password admin sesungguhnya.
    -- Contoh hash bcrypt 'Password123!' — HARUS DIGANTI DBA SEBELUM DEPLOY PRODUKSI.
    '$2b$12$LJ3m4ys3Lk0TSwHCpNqrAOZBXK8mB3yZF5s0HVMCJzmVCAg8FwvKe',
    'active',
    'admin',
    now(),
    'v1'
) ON CONFLICT (email) DO NOTHING;
