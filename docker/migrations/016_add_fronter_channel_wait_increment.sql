ALTER TABLE users
ADD COLUMN fronter_channel_wait_increment INTEGER DEFAULT 100 CHECK (fronter_channel_wait_increment >= 100);
