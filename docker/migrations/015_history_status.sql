CREATE TABLE history_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE users
ADD COLUMN history_limit INTEGER CHECK (history_limit >= 0),
ADD COLUMN history_truncate_after_days INTEGER CHECK (history_truncate_after_days >= 0);