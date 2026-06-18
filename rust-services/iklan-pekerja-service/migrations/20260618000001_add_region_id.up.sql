ALTER TABLE iklan_pekerja.iklan ADD COLUMN region_id TEXT;
CREATE INDEX idx_pekerja_region ON iklan_pekerja.iklan (region_id) WHERE region_id IS NOT NULL;
