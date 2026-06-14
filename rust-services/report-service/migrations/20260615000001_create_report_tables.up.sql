CREATE SCHEMA IF NOT EXISTS report;

CREATE TABLE report.report (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id         UUID        NOT NULL,
    target_type         TEXT        NOT NULL CHECK (target_type IN ('iklan', 'user')),
    target_id           UUID        NOT NULL,
    keterangan          TEXT        NOT NULL,
    evidence_object_key TEXT,
    status              TEXT        NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_review', 'rejected', 'resolved')),
    action_note         TEXT,
    reviewed_by         UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_report_status ON report.report (status);
CREATE INDEX idx_report_reporter_id ON report.report (reporter_id);
CREATE INDEX idx_report_target_id ON report.report (target_id);
