-- Internet plan for the player's VM (My Computer). Informational; default 'basic'.
ALTER TABLE vms ADD COLUMN IF NOT EXISTS internet_plan_id VARCHAR(32) NOT NULL DEFAULT 'basic';
ALTER TABLE vms ADD COLUMN IF NOT EXISTS internet_plan_next_billing_ms BIGINT;
