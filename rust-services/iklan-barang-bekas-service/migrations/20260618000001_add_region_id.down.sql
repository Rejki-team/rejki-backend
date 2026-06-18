DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_bekas_region;
ALTER TABLE iklan_barang_bekas.iklan DROP COLUMN IF EXISTS region_id;
