-- Per-card transaction history.
-- tx_type values: 'purchase', 'payment', 'refund'
CREATE TABLE IF NOT EXISTS wallet_card_transactions (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    card_id     UUID        NOT NULL REFERENCES wallet_cards(id) ON DELETE CASCADE,
    player_id   UUID        NOT NULL REFERENCES players(id),
    tx_type     VARCHAR(20) NOT NULL,
    amount      BIGINT      NOT NULL,  -- in cents
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_card_txn_card_time ON wallet_card_transactions(card_id, created_at DESC);
