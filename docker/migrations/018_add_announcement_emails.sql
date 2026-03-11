CREATE TABLE IF NOT EXISTS announcement_email_definitions (
    email_id VARCHAR(255) PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pending_emails (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email_id VARCHAR(255) NOT NULL REFERENCES announcement_email_definitions(email_id) ON DELETE CASCADE,
    last_attempt TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, email_id)
);
