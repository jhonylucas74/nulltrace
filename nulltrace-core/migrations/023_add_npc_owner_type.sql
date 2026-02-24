-- Add 'npc' owner_type for accounts identified by stable account_id (e.g. money.null).
-- Wallet logic uses keys; vm_id is never used. NPC accounts use deterministic owner_id from account_id.
ALTER TABLE fkebank_accounts DROP CONSTRAINT IF EXISTS fkebank_accounts_owner_type_check;
ALTER TABLE fkebank_accounts ADD CONSTRAINT fkebank_accounts_owner_type_check
  CHECK (owner_type IN ('player', 'vm', 'npc'));
