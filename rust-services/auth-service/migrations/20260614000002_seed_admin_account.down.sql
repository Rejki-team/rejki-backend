-- Rollback seed admin: hapus akun admin yang di-seed (hanya bila email cocok).
-- Aman — tidak menghapus akun admin yang dibuat manual oleh DBA di luar migrasi seed.
DELETE FROM auth.users WHERE email = 'admin@rejki.id';
