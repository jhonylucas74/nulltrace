-- Fix cards with credit_limit = 0 (legacy or bug) to use default $200.
-- Skip if credit_limit was already dropped by 027 (idempotent).
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'public' AND table_name = 'wallet_cards' AND column_name = 'credit_limit') THEN
    UPDATE wallet_cards SET credit_limit = 20000 WHERE credit_limit = 0;
  END IF;
END $$;
