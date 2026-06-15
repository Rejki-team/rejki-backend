-- Pelatihan badge: sertifikat yang diajukan peserta, diverifikasi admin.
-- Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D4

CREATE TABLE IF NOT EXISTS iklan_pelatihan.pelatihan_badge (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pelatihan_id UUID NOT NULL,
    user_id UUID NOT NULL,
    sertifikat_object_key TEXT,
    approved_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_review', 'rejected', 'approved')),
    reviewed_by UUID,
    review_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_badge_pelatihan
    ON iklan_pelatihan.pelatihan_badge (pelatihan_id);
CREATE INDEX IF NOT EXISTS idx_badge_user
    ON iklan_pelatihan.pelatihan_badge (user_id);
CREATE INDEX IF NOT EXISTS idx_badge_status
    ON iklan_pelatihan.pelatihan_badge (status);
