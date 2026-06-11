CREATE TABLE IF NOT EXISTS user_svc.profiles (
    id         UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    auth_id    UUID        NOT NULL UNIQUE, -- denormalized dari auth.users, no FK
    username   TEXT        NOT NULL UNIQUE,
    full_name  TEXT,
    avatar     TEXT,
    bio        TEXT,
    phone      TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_profiles_auth_id ON user_svc.profiles (auth_id);
