-- Add moderation_status + deleted_at (soft-delete) for iklan_barang_bekas.
-- Note: foto_urls already exists in the schema, only moderation fields are new.
-- Ref: openspec/changes/extend-iklan-moderation/design.md D1, D5

ALTER TABLE iklan_barang_bekas.iklan
    ADD COLUMN IF NOT EXISTS moderation_status TEXT NOT NULL DEFAULT 'active'
        CHECK (moderation_status IN ('active', 'suspended_temp', 'suspended_permanent')),
    ADD COLUMN IF NOT EXISTS deleted_at        TIMESTAMPTZ;

-- Index for admin listing filtered/sorted by moderation status.
CREATE INDEX IF NOT EXISTS idx_barang_moderation
    ON iklan_barang_bekas.iklan (moderation_status)
    WHERE deleted_at IS NULL;

-- Replace the old is_sold-only index with a combined one for public listing.
DROP INDEX IF EXISTS iklan_barang_bekas.idx_barang_bekas_created;
CREATE INDEX idx_barang_created_active
    ON iklan_barang_bekas.iklan (created_at DESC)
    WHERE is_sold = false AND moderation_status = 'active' AND deleted_at IS NULL;
