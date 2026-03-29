-- we had accidentally set these values to false. they should be true.

ALTER TABLE users ALTER COLUMN from_pluralkit_prefer_displayname SET DEFAULT true;
ALTER TABLE users ALTER COLUMN from_pluralkit_respect_member_visibility SET DEFAULT true;
ALTER TABLE users ALTER COLUMN from_pluralkit_respect_field_visibility SET DEFAULT true;


UPDATE users SET
    from_pluralkit_prefer_displayname = true,
    from_pluralkit_respect_member_visibility = true,
    from_pluralkit_respect_field_visibility = true;
