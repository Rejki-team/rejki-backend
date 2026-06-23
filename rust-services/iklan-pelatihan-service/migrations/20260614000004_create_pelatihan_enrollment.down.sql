-- Rollback pelatihan enrollment table.

DROP INDEX IF EXISTS iklan_pelatihan.idx_enrollment_pelatihan;
DROP INDEX IF EXISTS iklan_pelatihan.idx_enrollment_user;
DROP INDEX IF EXISTS iklan_pelatihan.idx_enrollment_status;

DROP TABLE IF EXISTS iklan_pelatihan.pelatihan_enrollment;
