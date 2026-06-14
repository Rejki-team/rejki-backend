-- Rollback pelatihan badge table.

DROP INDEX IF EXISTS iklan_pelatihan.idx_badge_pelatihan;
DROP INDEX IF EXISTS iklan_pelatihan.idx_badge_user;
DROP INDEX IF EXISTS iklan_pelatihan.idx_badge_status;

DROP TABLE IF EXISTS iklan_pelatihan.pelatihan_badge;
