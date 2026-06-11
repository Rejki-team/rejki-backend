CREATE TABLE IF NOT EXISTS chat.conversations (
    id         UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    user_a     UUID        NOT NULL,
    user_b     UUID        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_a, user_b) -- user_a selalu < user_b (di-enforce di application layer)
);

CREATE TABLE IF NOT EXISTS chat.messages (
    id              UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    conversation_id UUID        NOT NULL,
    sender_id       UUID        NOT NULL,
    content         TEXT        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_messages_conv_created ON chat.messages (conversation_id, created_at DESC);
