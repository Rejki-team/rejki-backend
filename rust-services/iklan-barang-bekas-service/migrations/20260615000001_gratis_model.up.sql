-- Migrasi model jual-beli → gratis/donasi (extend-barang-bekas-gratis-model).
-- Ref: openspec/changes/extend-barang-bekas-gratis-model/design.md D1-D2
--
-- Perubahan:
--   1. DROP kolom jual-beli: harga, kondisi, is_sold
--   2. ADD  kolom donasi:  jenis_barang (bekas|baru), jumlah (>=1),
--                          lokasi_pengambilan, availability_status (tersedia|sudah_diambil)
--   3. Ganti index idx_barang_bekas_created yg depend on is_sold

-- 1. Hapus index dependen sebelum drop kolom
DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_bekas_created;

-- 2. Drop kolom model jual-beli
ALTER TABLE iklan_barang_bekas.iklan
    DROP COLUMN IF EXISTS harga,
    DROP COLUMN IF EXISTS kondisi,
    DROP COLUMN IF EXISTS is_sold;

-- 3. Tambah kolom model gratis/donasi
ALTER TABLE iklan_barang_bekas.iklan
    ADD COLUMN IF NOT EXISTS jenis_barang         TEXT   NOT NULL DEFAULT 'bekas'
        CHECK (jenis_barang IN ('bekas', 'baru')),
    ADD COLUMN IF NOT EXISTS jumlah               INT    NOT NULL DEFAULT 1
        CHECK (jumlah >= 1),
    ADD COLUMN IF NOT EXISTS lokasi_pengambilan    TEXT   NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS availability_status   TEXT   NOT NULL DEFAULT 'tersedia'
        CHECK (availability_status IN ('tersedia', 'sudah_diambil'));

-- 4. Index baru — sort default list/listing admin tetap pakai created_at DESC
CREATE INDEX IF NOT EXISTS idx_barang_bekas_created
    ON iklan_barang_bekas.iklan (created_at DESC);
