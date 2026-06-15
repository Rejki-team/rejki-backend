-- Add moderation_status + deleted_at (soft-delete) + foto_urls for iklan_pelatihan.
-- Ref: openspec/changes/extend-iklan-moderation/design.md D1, D5

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN IF NOT EXISTS moderation_status TEXT NOT NULL DEFAULT 'active'
        CHECK (moderation_status IN ('active', 'suspended_temp', 'suspended_permanent')),
    ADD COLUMN IF NOT EXISTS deleted_at        TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS foto_urls         TEXT[] NOT NULL DEFAULT '{}';

-- Index for admin listing filtered/sorted by moderation status.
CREATE INDEX IF NOT EXISTS idx_pelatihan_moderation
    ON iklan_pelatihan.iklan (moderation_status)
    WHERE deleted_at IS NULL;

-- Replace the old is_active-only index with a combined one for public listing.
DROP INDEX IF EXISTS iklan_pelatihan.idx_pelatihan_created;
CREATE INDEX idx_pelatihan_created_active
    ON iklan_pelatihan.iklan (created_at DESC)
    WHERE is_active = true AND moderation_status = 'active' AND deleted_at IS NULL;
