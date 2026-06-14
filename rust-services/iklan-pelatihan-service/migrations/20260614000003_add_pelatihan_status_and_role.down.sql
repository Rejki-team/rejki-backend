-- Rollback 7-stage lifecycle status + created_by_role + jumlah_peserta.

DROP INDEX IF EXISTS iklan_pelatihan.idx_pelatihan_status;

ALTER TABLE iklan_pelatihan.iklan
    DROP COLUMN IF EXISTS status,
    DROP COLUMN IF EXISTS created_by_role,
    DROP COLUMN IF EXISTS jumlah_peserta;
