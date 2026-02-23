-- Fkebank USD accounts: one per owner (player or VM). Key = PIX-style identifier.
-- Balance in cents (BIGINT). No player_id on transactions; identity is the key.
CREATE TABLE IF NOT EXISTS fkebank_accounts (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    owner_type  VARCHAR(10) NOT NULL CHECK (owner_type IN ('player', 'vm')),
    owner_id    UUID        NOT NULL,
    key         TEXT        NOT NULL UNIQUE,
    full_name   TEXT,
    document_id TEXT,
    balance     BIGINT      NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_type, owner_id)
);

CREATE INDEX IF NOT EXISTS idx_fkebank_accounts_key ON fkebank_accounts(key);
CREATE INDEX IF NOT EXISTS idx_fkebank_accounts_owner ON fkebank_accounts(owner_type, owner_id);
