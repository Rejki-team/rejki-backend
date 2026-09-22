-- B-2: nomor telepon unik per akun. `phone` disimpan sebagai ciphertext AES-256-GCM
-- (nonce acak per enkripsi) sehingga TIDAK deterministik — UNIQUE langsung di kolom itu
-- tidak berfungsi. `phone_hash` adalah HMAC-SHA256 deterministik (kunci terpisah
-- DATA_HASH_KEY, lihat common/crypto::hash_lookup) dipakai khusus untuk uniqueness lookup.
--
-- Kolom nullable dulu (baris lama tanpa phone_hash tetap valid histori, sesuai konvensi
-- migration §4.3). Unique index PARTIAL (WHERE phone_hash IS NOT NULL) agar baris lama
-- yang belum di-backfill tidak saling bentrok sebagai "sama-sama NULL".
ALTER TABLE auth.users
    ADD COLUMN IF NOT EXISTS phone_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS uq_auth_users_phone_hash
    ON auth.users (phone_hash)
    WHERE phone_hash IS NOT NULL;
