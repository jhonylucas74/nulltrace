CREATE TABLE IF NOT EXISTS vm_users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    vm_id         UUID NOT NULL REFERENCES vms(id) ON DELETE CASCADE,
    username      VARCHAR(64) NOT NULL,
    uid           INT NOT NULL,
    home_dir      VARCHAR(255) NOT NULL DEFAULT '/home',
    shell         VARCHAR(255) NOT NULL DEFAULT '/bin/sh',
    password_hash VARCHAR(128),
    is_root       BOOLEAN NOT NULL DEFAULT false,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE(vm_id, username),
    UNIQUE(vm_id, uid)
);

CREATE INDEX IF NOT EXISTS idx_vm_users_vm_id ON vm_users(vm_id);
