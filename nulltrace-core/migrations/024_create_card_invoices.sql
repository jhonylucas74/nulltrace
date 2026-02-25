-- Card invoices: merchant creates invoice, buyer pays with card. 5% fee is lost (provider simulation).
CREATE TABLE IF NOT EXISTS card_invoices (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    destination_key TEXT        NOT NULL,
    amount_cents    BIGINT      NOT NULL,
    fee_percent     INT         NOT NULL DEFAULT 5,
    status          TEXT        NOT NULL DEFAULT 'pending',
    paid_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_card_invoices_status ON card_invoices(status);
