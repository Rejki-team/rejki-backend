DROP TABLE IF EXISTS iklan_pekerjaan.lamaran;
DROP INDEX IF EXISTS iklan_pekerjaan.idx_pekerjaan_status;
ALTER TABLE iklan_pekerjaan.iklan DROP COLUMN IF EXISTS status;
