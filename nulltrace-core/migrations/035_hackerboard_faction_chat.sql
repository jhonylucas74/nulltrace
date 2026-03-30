-- Hackerboard Phase 2.2: faction group chat (one room per faction).
CREATE TABLE IF NOT EXISTS hackerboard_faction_messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    faction_id      UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
    from_player_id  UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    body            TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_hackerboard_faction_msg_faction_created
    ON hackerboard_faction_messages (faction_id, created_at DESC);
