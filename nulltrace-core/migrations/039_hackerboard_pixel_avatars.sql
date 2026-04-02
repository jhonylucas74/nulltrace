-- Hackerboard avatars: validated NTPX binary blob (VM file format), copied at set time.
ALTER TABLE players ADD COLUMN IF NOT EXISTS hackerboard_avatar_pixel BYTEA NULL;
ALTER TABLE factions ADD COLUMN IF NOT EXISTS hackerboard_emblem_pixel BYTEA NULL;
