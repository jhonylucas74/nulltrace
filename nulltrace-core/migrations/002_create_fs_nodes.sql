CREATE TABLE IF NOT EXISTS fs_nodes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vm_id       UUID NOT NULL REFERENCES vms(id) ON DELETE CASCADE,
    parent_id   UUID REFERENCES fs_nodes(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    node_type   VARCHAR(10) NOT NULL CHECK (node_type IN ('file', 'directory')),
    mime_type   VARCHAR(127),
    size_bytes  BIGINT NOT NULL DEFAULT 0,
    permissions VARCHAR(10) NOT NULL DEFAULT 'rwxr-xr-x',
    owner       VARCHAR(64) NOT NULL DEFAULT 'root',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE(vm_id, parent_id, name)
);

CREATE INDEX IF NOT EXISTS idx_fs_nodes_vm_id ON fs_nodes(vm_id);
CREATE INDEX IF NOT EXISTS idx_fs_nodes_parent ON fs_nodes(parent_id);
