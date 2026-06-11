CREATE TABLE IF NOT EXISTS notification.notifications (
    id           UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    recipient_id UUID        NOT NULL,
    title        TEXT        NOT NULL,
    body         TEXT        NOT NULL,
    data         JSONB,
    is_read      BOOLEAN     NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_notif_recipient_created ON notification.notifications (recipient_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notif_unread ON notification.notifications (recipient_id) WHERE is_read = false;
