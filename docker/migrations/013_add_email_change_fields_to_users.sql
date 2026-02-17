ALTER TABLE users
ADD COLUMN new_email TEXT NULL,
ADD COLUMN email_verification_token_hash TEXT NULL,
ADD COLUMN email_verification_token_expires_at TIMESTAMPTZ NULL;