ALTER TABLE iklan_pekerjaan.iklan ADD COLUMN region_id TEXT;
CREATE INDEX idx_pekerjaan_region ON iklan_pekerjaan.iklan (region_id) WHERE region_id IS NOT NULL;
