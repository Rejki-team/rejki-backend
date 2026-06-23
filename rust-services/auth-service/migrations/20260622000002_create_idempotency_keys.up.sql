-- W3D-14 (D4): Idempotency keys untuk POST kritis
-- TTL: 24 jam (auto-cleanup via query, bukan trigger)
-- Referensi: api-standard.html §Idempotency

CREATE TABLE auth.idempotency_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key             TEXT NOT NULL,
    request_hash    TEXT NOT NULL,
    response_status INTEGER,
    response_body   JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_idempotency_key UNIQUE (key)
);

CREATE INDEX idx_idempotency_keys_created_at
    ON auth.idempotency_keys (created_at);
