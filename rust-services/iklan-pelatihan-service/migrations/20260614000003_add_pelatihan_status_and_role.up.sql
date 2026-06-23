-- Add 7-stage lifecycle status + created_by_role + jumlah_peserta to iklan_pelatihan.
-- Ref: openspec/changes/add-pelatihan-enrollment-badge/design.md D1, D2, D6

ALTER TABLE iklan_pelatihan.iklan
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'verifikasi_tertunda'
        CHECK (status IN (
            'verifikasi_tertunda',
            'verifikasi_dalam_proses',
            'verifikasi_ditolak',
            'verifikasi_diterima',
            'pelatihan_belum_dimulai',
            'pelatihan_berjalan',
            'pelatihan_selesai'
        )),
    ADD COLUMN IF NOT EXISTS created_by_role TEXT NOT NULL DEFAULT 'user'
        CHECK (created_by_role IN ('admin', 'user')),
    ADD COLUMN IF NOT EXISTS jumlah_peserta INTEGER;

-- Index for filtering/sorting by the 7-stage status.
CREATE INDEX IF NOT EXISTS idx_pelatihan_status
    ON iklan_pelatihan.iklan (status)
    WHERE deleted_at IS NULL;

-- Update existing rows: set status based on moderation_status for backward compat.
-- Active moderation → actively-displayed training (pelatihan_berjalan as reasonable default).
-- Suspended temp → verifikasi_tertunda (needs re-review).
-- Suspended permanent → verifikasi_ditolak (terminal reject).
UPDATE iklan_pelatihan.iklan
SET status = CASE moderation_status
    WHEN 'active' THEN 'pelatihan_berjalan'
    WHEN 'suspended_temp' THEN 'verifikasi_tertunda'
    WHEN 'suspended_permanent' THEN 'verifikasi_ditolak'
    ELSE 'verifikasi_tertunda'
END
WHERE status = 'verifikasi_tertunda';
