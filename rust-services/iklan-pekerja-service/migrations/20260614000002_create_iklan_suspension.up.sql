-- Riwayat suspend iklan (audit trail).
-- Ref: openspec/changes/extend-iklan-moderation/design.md D2

CREATE TABLE IF NOT EXISTS iklan_pekerja.iklan_suspension (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    iklan_id            UUID        NOT NULL,
    is_permanent        BOOLEAN     NOT NULL DEFAULT false,
    reason              TEXT        NOT NULL,
    evidence_object_key TEXT,
    expires_at          TIMESTAMPTZ,
    created_by          UUID        NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pekerja_suspension_iklan
    ON iklan_pekerja.iklan_suspension (iklan_id);
