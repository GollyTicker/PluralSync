ALTER TABLE users ADD COLUMN enable_from_pluralkit BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN enc__from_pluralkit_webhook_signing_token bytea;
