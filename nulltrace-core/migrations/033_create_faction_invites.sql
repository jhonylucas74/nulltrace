-- Pending faction membership invites (Phase 1.1).
CREATE TABLE IF NOT EXISTS faction_invites (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    faction_id      UUID NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
    from_player_id  UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    to_player_id    UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    status          VARCHAR(16) NOT NULL CHECK (status IN ('pending', 'accepted', 'declined', 'cancelled')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_faction_invites_one_pending_per_target
    ON faction_invites (faction_id, to_player_id)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_faction_invites_to_pending
    ON faction_invites (to_player_id)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_faction_invites_from_pending
    ON faction_invites (from_player_id)
    WHERE status = 'pending';
