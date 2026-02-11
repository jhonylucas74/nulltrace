CREATE TABLE IF NOT EXISTS fs_contents (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    node_id     UUID NOT NULL UNIQUE REFERENCES fs_nodes(id) ON DELETE CASCADE,
    data        BYTEA NOT NULL,
    checksum    VARCHAR(64)
);
