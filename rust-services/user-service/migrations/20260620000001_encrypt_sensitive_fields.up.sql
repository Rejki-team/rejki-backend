-- W3C-09: Enkripsi at rest — phone + rekening bank
-- Kolom baru (tidak destruktif): phone_encrypted TEXT (base64 AES-256-GCM ciphertext),
-- rekening_encrypted TEXT (base64 AES-256-GCM ciphertext dari JSON {bank, number, holder}).
-- Kolom `phone` lama dipertahankan sebagai fallback sampai backfill selesai.

ALTER TABLE user_svc.profiles
    ADD COLUMN IF NOT EXISTS phone_encrypted   TEXT,
    ADD COLUMN IF NOT EXISTS rekening_encrypted TEXT;
