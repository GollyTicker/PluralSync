CREATE TABLE IF NOT EXISTS email_rate_limit (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    current_day DATE NOT NULL,
    count INTEGER NOT NULL DEFAULT 0
);

INSERT INTO email_rate_limit (id, current_day, count) VALUES (1, CURRENT_DATE, 0)
ON CONFLICT (id) DO NOTHING;
