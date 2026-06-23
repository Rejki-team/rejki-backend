CREATE SCHEMA IF NOT EXISTS comms;

CREATE TABLE comms.corporate_article (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id        UUID        NOT NULL,
    category         TEXT        NOT NULL DEFAULT 'informasi' CHECK (category IN ('informasi')),
    title            TEXT        NOT NULL,
    body             TEXT        NOT NULL,
    photo_object_key TEXT,
    deleted_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_comms_article_category ON comms.corporate_article (category);
