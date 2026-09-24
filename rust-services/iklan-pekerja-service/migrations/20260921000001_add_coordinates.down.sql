DROP INDEX IF EXISTS iklan_pekerja.idx_iklan_pekerja_coordinates;

ALTER TABLE iklan_pekerja.iklan
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;
