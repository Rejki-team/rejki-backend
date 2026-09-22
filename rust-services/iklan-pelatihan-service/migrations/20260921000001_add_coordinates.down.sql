DROP INDEX IF EXISTS iklan_pelatihan.idx_iklan_pelatihan_coordinates;

ALTER TABLE iklan_pelatihan.iklan
    DROP COLUMN IF EXISTS latitude,
    DROP COLUMN IF EXISTS longitude;
