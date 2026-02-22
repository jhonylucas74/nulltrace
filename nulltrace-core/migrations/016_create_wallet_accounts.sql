-- Wallet accounts: one row per player per currency, balance in cents (integer).
-- Divide by 100 on the frontend for display.
CREATE TABLE IF NOT EXISTS wallet_accounts (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    player_id   UUID        NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    currency    VARCHAR(10) NOT NULL,  -- 'USD', 'BTC', 'ETH', 'SOL'
    balance     BIGINT      NOT NULL DEFAULT 0,  -- in cents; never negative
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (player_id, currency)
);

CREATE INDEX IF NOT EXISTS idx_wallet_accounts_player ON wallet_accounts(player_id);
