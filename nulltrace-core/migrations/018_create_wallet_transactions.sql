-- Wallet transactions ledger.
-- tx_type values: 'credit', 'debit', 'transfer_in', 'transfer_out', 'convert'
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id                      UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    player_id               UUID        NOT NULL REFERENCES players(id),
    tx_type                 VARCHAR(20) NOT NULL,
    currency                VARCHAR(10) NOT NULL,
    amount                  BIGINT      NOT NULL,  -- in cents
    fee                     BIGINT      NOT NULL DEFAULT 0,
    description             TEXT,
    counterpart_address     TEXT,         -- destination/source address
    counterpart_player_id   UUID        REFERENCES players(id),
    related_transaction_id  UUID        REFERENCES wallet_transactions(id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wallet_txn_player_time ON wallet_transactions(player_id, created_at DESC);
