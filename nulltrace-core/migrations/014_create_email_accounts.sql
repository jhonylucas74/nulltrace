-- email_accounts: players and NPCs share the same table.
-- player_id is NULL for VMs without a player owner.
CREATE TABLE IF NOT EXISTS email_accounts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id     UUID REFERENCES players(id) ON DELETE CASCADE,
    email_address TEXT NOT NULL UNIQUE,
    token         TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_email_accounts_player  ON email_accounts(player_id);
CREATE INDEX IF NOT EXISTS idx_email_accounts_address ON email_accounts(email_address);
