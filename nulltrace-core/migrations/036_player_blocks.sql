-- Hackerboard: one-way block; feed/ranking/DMs hide either direction for the viewer.
CREATE TABLE IF NOT EXISTS player_blocks (
    blocker_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    blocked_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CONSTRAINT player_blocks_no_self CHECK (blocker_id <> blocked_id)
);

CREATE INDEX IF NOT EXISTS idx_player_blocks_blocker ON player_blocks (blocker_id);
CREATE INDEX IF NOT EXISTS idx_player_blocks_blocked ON player_blocks (blocked_id);
