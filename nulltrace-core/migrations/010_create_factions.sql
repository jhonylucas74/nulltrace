-- Factions: optional group for players. Player may or may not be in a faction.
CREATE TABLE IF NOT EXISTS factions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(128) NOT NULL,
    creator_id  UUID REFERENCES players(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_factions_creator_id ON factions(creator_id);

-- Optional: player can belong to at most one faction.
ALTER TABLE players ADD COLUMN IF NOT EXISTS faction_id UUID REFERENCES factions(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_players_faction_id ON players(faction_id);
