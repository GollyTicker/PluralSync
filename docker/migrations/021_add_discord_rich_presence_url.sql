CREATE TYPE discord_rich_presence_url AS ENUM ('None', 'PluralSyncAboutPage', 'PluralSyncFrontingWebsiteIfDefined', 'CustomUrl');

ALTER TABLE users
ADD COLUMN discord_rich_presence_url discord_rich_presence_url NOT NULL DEFAULT 'PluralSyncFrontingWebsiteIfDefined',
ADD COLUMN discord_rich_presence_url_custom TEXT;
