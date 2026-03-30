-- Hackerboard Phase 2.1: direct messages between players.
CREATE TABLE IF NOT EXISTS hackerboard_dm_messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_player_id  UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    to_player_id    UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    body            TEXT NOT NULL CHECK (char_length(body) BETWEEN 1 AND 4000),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_hackerboard_dm_from_created
    ON hackerboard_dm_messages (from_player_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_hackerboard_dm_to_created
    ON hackerboard_dm_messages (to_player_id, created_at DESC);
