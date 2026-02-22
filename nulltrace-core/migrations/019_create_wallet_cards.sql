-- Virtual credit cards issued by Fkebank.
-- billing_day_of_week = 1 means Monday (ISO weekday).
-- All monetary fields in cents (BIGINT).
CREATE TABLE IF NOT EXISTS wallet_cards (
    id                  UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    player_id           UUID        NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    label               VARCHAR(100),
    number_full         VARCHAR(16) NOT NULL,
    last4               VARCHAR(4)  NOT NULL,
    expiry_month        INT         NOT NULL,
    expiry_year         INT         NOT NULL,
    cvv                 VARCHAR(4)  NOT NULL,
    holder_name         TEXT        NOT NULL,
    credit_limit        BIGINT      NOT NULL DEFAULT 100000,  -- $1,000.00
    current_debt        BIGINT      NOT NULL DEFAULT 0,
    is_virtual          BOOLEAN     NOT NULL DEFAULT TRUE,
    is_active           BOOLEAN     NOT NULL DEFAULT TRUE,
    billing_day_of_week INT         NOT NULL DEFAULT 1,       -- 1 = Monday
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_wallet_cards_player ON wallet_cards(player_id);
