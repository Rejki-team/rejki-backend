-- Kolom jam kerja (F-5, Kelompok 6 P7.1) — gap kolom admin table, PRD §6.5.
-- Nullable: baris existing belum punya nilai ini.
ALTER TABLE iklan_pekerjaan.iklan
    ADD COLUMN IF NOT EXISTS jam_kerja TEXT;
