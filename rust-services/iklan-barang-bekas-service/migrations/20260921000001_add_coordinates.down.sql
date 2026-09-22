DROP INDEX IF EXISTS iklan_barang_bekas.idx_iklan_barang_bekas_coordinates;

ALTER TABLE iklan_barang_bekas.iklan
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;
