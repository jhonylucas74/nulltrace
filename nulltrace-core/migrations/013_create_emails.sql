CREATE TABLE IF NOT EXISTS emails (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_address TEXT NOT NULL,
    to_address   TEXT NOT NULL,
    subject      TEXT NOT NULL DEFAULT '',
    body         TEXT NOT NULL DEFAULT '',
    folder       TEXT NOT NULL DEFAULT 'inbox',
    read         BOOLEAN NOT NULL DEFAULT false,
    sent_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_emails_to_folder ON emails(to_address, folder);
