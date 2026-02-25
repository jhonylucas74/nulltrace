-- Account-level credit limit shared by all cards. One row per player.
CREATE TABLE IF NOT EXISTS player_credit_accounts (
    player_id       UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
    credit_limit    BIGINT NOT NULL DEFAULT 20000,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
