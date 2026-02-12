-- Add points to players for ranking.
ALTER TABLE players ADD COLUMN IF NOT EXISTS points INTEGER NOT NULL DEFAULT 0;
