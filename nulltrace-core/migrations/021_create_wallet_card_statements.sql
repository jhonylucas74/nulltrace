-- Weekly billing statements for each card.
-- status: 'open' (current cycle), 'closed' (awaiting payment), 'paid'
-- Cards are billed every Monday; due_date = next Monday after period_end.
CREATE TABLE IF NOT EXISTS wallet_card_statements (
    id           UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    card_id      UUID        NOT NULL REFERENCES wallet_cards(id) ON DELETE CASCADE,
    period_start TIMESTAMPTZ NOT NULL,
    period_end   TIMESTAMPTZ NOT NULL,
    total_amount BIGINT      NOT NULL DEFAULT 0,  -- in cents
    status       VARCHAR(20) NOT NULL DEFAULT 'open',
    due_date     TIMESTAMPTZ NOT NULL,
    paid_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_card_stmt_card ON wallet_card_statements(card_id);
