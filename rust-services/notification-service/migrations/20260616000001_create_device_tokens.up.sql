CREATE TABLE IF NOT EXISTS notification.device_tokens (
    id         UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id    UUID        NOT NULL,
    token      TEXT        NOT NULL,
    platform   TEXT        NOT NULL CHECK (platform IN ('android', 'ios', 'web')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_device_tokens_token ON notification.device_tokens (token);
CREATE INDEX IF NOT EXISTS idx_device_tokens_user_id ON notification.device_tokens (user_id);
