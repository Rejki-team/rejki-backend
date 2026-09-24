-- Kolom jam kerja & nomor kontak pekerja (F-7, Kelompok 6 P7.2/P7.3).
-- `phone_number` SUDAH dikirim mobile (create/update) tapi selama ini dibuang
-- diam-diam oleh backend (tidak ada di entity/DTO sama sekali) — bukan salah
-- label `lokasi`, field ini murni belum ada. Nullable: baris existing.
ALTER TABLE iklan_pekerja.iklan
    ADD COLUMN IF NOT EXISTS jam_kerja TEXT,
    ADD COLUMN IF NOT EXISTS phone_number TEXT;
