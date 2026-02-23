-- One token per Fkebank account to authorize transfers (e.g. from Lua/VM).
CREATE TABLE IF NOT EXISTS fkebank_tokens (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    account_id  UUID        NOT NULL REFERENCES fkebank_accounts(id) ON DELETE CASCADE,
    token       TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (account_id)
);

CREATE INDEX IF NOT EXISTS idx_fkebank_tokens_account ON fkebank_tokens(account_id);
