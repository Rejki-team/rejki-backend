ALTER TABLE iklan_pelatihan.iklan ADD COLUMN region_id TEXT;
CREATE INDEX idx_pelatihan_region ON iklan_pelatihan.iklan (region_id) WHERE region_id IS NOT NULL;
