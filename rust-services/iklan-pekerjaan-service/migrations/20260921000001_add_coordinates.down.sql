DROP INDEX IF EXISTS iklan_pekerjaan.idx_iklan_pekerjaan_coordinates;

ALTER TABLE iklan_pekerjaan.iklan
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;
