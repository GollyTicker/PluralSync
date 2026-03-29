ALTER TABLE users ADD COLUMN from_pluralkit_prefer_displayname BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN from_pluralkit_respect_member_visibility BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN from_pluralkit_respect_field_visibility BOOLEAN NOT NULL DEFAULT false;
