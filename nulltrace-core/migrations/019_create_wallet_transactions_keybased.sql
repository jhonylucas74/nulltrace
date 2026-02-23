-- Key-based transactions: from_key / to_key (PIX key for USD, address for crypto). No player_id/vm_id.
-- Drop old table if it existed (previous schema had player_id).
DROP TABLE IF EXISTS wallet_transactions;
CREATE TABLE wallet_transactions (
    id              UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    currency        VARCHAR(10) NOT NULL,
    amount          BIGINT      NOT NULL,
    fee             BIGINT      NOT NULL DEFAULT 0,
    description     TEXT,
    from_key        TEXT        NOT NULL,
    to_key          TEXT        NOT NULL,
    counterpart_key TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wallet_txn_from_key ON wallet_transactions(from_key, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_wallet_txn_to_key ON wallet_transactions(to_key, created_at DESC);
