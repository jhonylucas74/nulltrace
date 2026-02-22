-- Wallet receive keys / addresses: one per player per currency.
-- USD  → fkebank-{32 hex}
-- BTC  → bc1q{38 bech32 chars}
-- ETH  → 0x{40 hex}
-- SOL  → {44 base58 chars}
CREATE TABLE IF NOT EXISTS wallet_keys (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    player_id   UUID        NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    currency    VARCHAR(10) NOT NULL,
    key_address TEXT        NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (player_id, currency)
);

CREATE INDEX IF NOT EXISTS idx_wallet_keys_player   ON wallet_keys(player_id);
CREATE INDEX IF NOT EXISTS idx_wallet_keys_address  ON wallet_keys(key_address);
