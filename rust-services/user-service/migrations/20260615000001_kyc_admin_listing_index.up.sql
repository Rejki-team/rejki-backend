-- Index pendukung listing pengajuan KYC untuk admin (add-user-admin-management).
-- Listing memfilter berdasarkan `status` dan mengurutkan berdasarkan `created_at`.
-- Composite (status, created_at DESC) melayani WHERE status = $1 ORDER BY created_at
-- tanpa scan penuh tabel — eliminasi degradasi listing saat volume membesar.
-- Ref: openspec/changes/add-user-admin-management/tasks.md 1.2

CREATE INDEX IF NOT EXISTS idx_kyc_submission_status_created
    ON user_svc.kyc_submission (status, created_at DESC);
