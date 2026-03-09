CREATE TABLE IF NOT EXISTS codelab_progress (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id   UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    challenge_id TEXT NOT NULL,
    solved_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (player_id, challenge_id)
);

CREATE INDEX IF NOT EXISTS idx_codelab_progress_player ON codelab_progress(player_id);
