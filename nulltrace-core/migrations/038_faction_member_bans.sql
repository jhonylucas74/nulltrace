-- Players banned from rejoining a specific faction (creator-managed).
CREATE TABLE IF NOT EXISTS faction_member_bans (
    faction_id UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
    banned_player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (faction_id, banned_player_id)
);

CREATE INDEX IF NOT EXISTS idx_faction_member_bans_player ON faction_member_bans (banned_player_id);
