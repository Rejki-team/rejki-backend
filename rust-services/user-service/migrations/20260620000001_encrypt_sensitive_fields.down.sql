-- Rollback: hapus kolom enkripsi at-rest (data hilang — pastikan backfill sudah selesai)
ALTER TABLE user_svc.profiles
    DROP COLUMN IF EXISTS phone_encrypted,
    DROP COLUMN IF EXISTS rekening_encrypted;
