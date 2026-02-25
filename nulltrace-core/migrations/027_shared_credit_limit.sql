-- Backfill player_credit_accounts from existing wallet_cards (one row per player, use first card's limit).
-- Remove per-card limit; limit is now at account level.
-- Idempotent: skip if credit_limit was already dropped.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'wallet_cards' AND column_name = 'credit_limit') THEN
    INSERT INTO player_credit_accounts (player_id, credit_limit, created_at)
    SELECT DISTINCT ON (player_id) player_id, credit_limit, now()
    FROM wallet_cards
    ON CONFLICT (player_id) DO NOTHING;
    ALTER TABLE wallet_cards DROP COLUMN credit_limit;
  END IF;
END $$;
