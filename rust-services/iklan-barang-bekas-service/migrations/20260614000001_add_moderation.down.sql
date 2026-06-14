-- Rollback moderation fields for iklan_barang_bekas.

DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_moderation;
DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_created_active;

ALTER TABLE iklan_barang_bekas.iklan
    DROP COLUMN IF EXISTS moderation_status,
    DROP COLUMN IF EXISTS deleted_at;

CREATE INDEX idx_barang_bekas_created
    ON iklan_barang_bekas.iklan (created_at DESC) WHERE is_sold = false;
