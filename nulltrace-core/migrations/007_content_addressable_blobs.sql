-- Blob store: content by hash (SHA-256 hex = 64 chars)
CREATE TABLE IF NOT EXISTS blob_store (
    hash    VARCHAR(64) PRIMARY KEY,
    data    BYTEA NOT NULL
);

-- Add content_hash column (nullable during migration)
ALTER TABLE fs_contents ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64) REFERENCES blob_store(hash);
