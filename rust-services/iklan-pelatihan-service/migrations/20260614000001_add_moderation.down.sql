-- Rollback moderation fields for iklan_pelatihan.

DROP INDEX IF EXISTS iklan_pelatihan.idx_pelatihan_moderation;
DROP INDEX IF EXISTS iklan_pelatihan.idx_pelatihan_created_active;

ALTER TABLE iklan_pelatihan.iklan
    DROP COLUMN IF EXISTS moderation_status,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS foto_urls;

CREATE INDEX idx_pelatihan_created
    ON iklan_pelatihan.iklan (created_at DESC) WHERE is_active = true;
