-- Rollback moderation fields for iklan_pekerja.

DROP INDEX IF EXISTS iklan_pekerja.idx_pekerja_moderation;
DROP INDEX IF EXISTS iklan_pekerja.idx_pekerja_created_active;

ALTER TABLE iklan_pekerja.iklan
    DROP COLUMN IF EXISTS moderation_status,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS foto_urls;

CREATE INDEX idx_pekerja_created
    ON iklan_pekerja.iklan (created_at DESC) WHERE is_active = true;
