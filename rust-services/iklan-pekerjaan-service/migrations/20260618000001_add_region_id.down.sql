DROP INDEX IF EXISTS iklan_pekerjaan.idx_pekerjaan_region;
ALTER TABLE iklan_pekerjaan.iklan DROP COLUMN IF EXISTS region_id;
