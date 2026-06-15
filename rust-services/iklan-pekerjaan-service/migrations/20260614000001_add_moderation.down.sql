-- Rollback moderation fields for iklan_pekerjaan.

DROP INDEX IF EXISTS iklan_pekerjaan.idx_pekerjaan_moderation;
DROP INDEX IF EXISTS iklan_pekerjaan.idx_pekerjaan_created_active;

ALTER TABLE iklan_pekerjaan.iklan
    DROP COLUMN IF EXISTS moderation_status,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS foto_urls;

CREATE INDEX idx_pekerjaan_created
    ON iklan_pekerjaan.iklan (created_at DESC) WHERE is_active = true;
