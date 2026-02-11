CREATE TABLE IF NOT EXISTS vms (
    id          UUID PRIMARY KEY,
    hostname    VARCHAR(255) NOT NULL,

    -- Specs
    cpu_cores   SMALLINT NOT NULL DEFAULT 1,
    memory_mb   INT NOT NULL DEFAULT 512,
    disk_mb     INT NOT NULL DEFAULT 10240,

    -- Estado
    status      VARCHAR(16) NOT NULL DEFAULT 'stopped'
                CHECK (status IN ('running', 'stopped', 'crashed')),

    -- Rede
    ip          VARCHAR(15),
    subnet      VARCHAR(18),
    gateway     VARCHAR(15),
    mac         VARCHAR(17),

    -- Owner
    owner_id    UUID,

    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
