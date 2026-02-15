-- Per-player keyboard shortcut overrides (JSON object: actionId -> array of key strings).
CREATE TABLE IF NOT EXISTS player_shortcuts (
    player_id   UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
    overrides   JSONB NOT NULL DEFAULT '{}',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
