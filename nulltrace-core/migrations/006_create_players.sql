-- Players: real game players (login accounts). VMs can be owned by a player.
CREATE TABLE IF NOT EXISTS players (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      VARCHAR(64) NOT NULL UNIQUE,
    password_hash VARCHAR(128) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_players_username ON players(username);

-- FK from vms.owner_id to players.id (owner_id already exists in vms).
-- Use DROP + ADD with exception handling so concurrent migration runs (e.g. parallel tests) do not deadlock.
ALTER TABLE vms DROP CONSTRAINT IF EXISTS fk_vms_owner;
DO $$
BEGIN
    ALTER TABLE vms ADD CONSTRAINT fk_vms_owner
        FOREIGN KEY (owner_id) REFERENCES players(id) ON DELETE SET NULL;
EXCEPTION
    WHEN duplicate_object THEN NULL;  -- constraint already exists (e.g. another process added it)
END $$;
