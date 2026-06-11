-- Idempotent: CREATE TABLE IF NOT EXISTS
-- Dijalankan oleh Bun.js saat startup sebelum consumer loop dimulai.

CREATE TABLE IF NOT EXISTS notification.notifications (
    id           UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    recipient_id UUID        NOT NULL,
    event_type   TEXT        NOT NULL DEFAULT 'push',
    title        TEXT        NOT NULL,
    body         TEXT        NOT NULL,
    deep_link    TEXT,
    is_read      BOOLEAN     NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notif_recipient
    ON notification.notifications (recipient_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_notif_unread
    ON notification.notifications (recipient_id)
    WHERE is_read = false;

-- Idempotency tracking — mencegah double dispatch jika consumer crash sebelum XACK
-- Gunakan event_id (UUID dari Rust publisher) sebagai idempotency key
CREATE TABLE IF NOT EXISTS notification.processed_events (
    id             UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    event_id       TEXT        NOT NULL UNIQUE,
    processed_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_processed_events_event_id
    ON notification.processed_events (event_id);

-- TTL cleanup: hapus entry > 7 hari
-- DELETE FROM notification.processed_events WHERE processed_at < now() - INTERVAL '7 days';

-- Device tokens — FCM token per user per platform
CREATE TABLE IF NOT EXISTS notification.device_tokens (
    id         UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id    UUID        NOT NULL,
    token      TEXT        NOT NULL UNIQUE,
    platform   TEXT        NOT NULL CHECK (platform IN ('android', 'ios', 'web')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_device_tokens_user
    ON notification.device_tokens (user_id);

-- User preferences — opt-out per saluran
CREATE TABLE IF NOT EXISTS notification.user_preferences (
    id            UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id       UUID        NOT NULL UNIQUE,
    push_enabled  BOOLEAN     NOT NULL DEFAULT true,
    email_enabled BOOLEAN     NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_user_preferences_user_id
    ON notification.user_preferences (user_id);
