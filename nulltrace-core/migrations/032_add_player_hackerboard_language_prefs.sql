-- Hackerboard feed filter and compose post language (per player).
ALTER TABLE players ADD COLUMN IF NOT EXISTS hackerboard_feed_language_filter VARCHAR(16) NOT NULL DEFAULT 'all';
ALTER TABLE players ADD COLUMN IF NOT EXISTS hackerboard_post_language VARCHAR(16) NOT NULL DEFAULT 'en';
