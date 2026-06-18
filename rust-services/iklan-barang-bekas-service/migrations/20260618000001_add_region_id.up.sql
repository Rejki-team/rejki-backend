ALTER TABLE iklan_barang_bekas.iklan ADD COLUMN region_id TEXT;
CREATE INDEX idx_barang_bekas_region ON iklan_barang_bekas.iklan (region_id) WHERE region_id IS NOT NULL;
