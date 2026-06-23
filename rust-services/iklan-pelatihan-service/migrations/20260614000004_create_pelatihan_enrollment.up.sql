-- Pelatihan enrollment: peserta mendaftar + bukti transfer, admin review.
-- Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D3

CREATE TABLE IF NOT EXISTS iklan_pelatihan.pelatihan_enrollment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pelatihan_id UUID NOT NULL,
    user_id UUID NOT NULL,
    bukti_transfer_object_key TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_review', 'rejected', 'approved')),
    reviewed_by UUID,
    review_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_enrollment_pelatihan
    ON iklan_pelatihan.pelatihan_enrollment (pelatihan_id);
CREATE INDEX IF NOT EXISTS idx_enrollment_user
    ON iklan_pelatihan.pelatihan_enrollment (user_id);
CREATE INDEX IF NOT EXISTS idx_enrollment_status
    ON iklan_pelatihan.pelatihan_enrollment (status);
