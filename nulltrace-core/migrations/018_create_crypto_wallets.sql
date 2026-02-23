-- Crypto wallets: no owner. Identity is key_address/public key only; not traceable.
-- Private key lives only in VM files; not stored in DB.
CREATE TABLE IF NOT EXISTS crypto_wallets (
    id          UUID        NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    key_address TEXT        NOT NULL UNIQUE,
    public_key  TEXT,
    currency    VARCHAR(10) NOT NULL CHECK (currency IN ('BTC', 'ETH', 'SOL')),
    balance     BIGINT      NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_crypto_wallets_address ON crypto_wallets(key_address);
CREATE INDEX IF NOT EXISTS idx_crypto_wallets_currency ON crypto_wallets(currency);
