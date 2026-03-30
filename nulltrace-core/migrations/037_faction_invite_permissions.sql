-- When false, only faction creator may send invites.
ALTER TABLE factions
    ADD COLUMN IF NOT EXISTS allow_member_invites BOOLEAN NOT NULL DEFAULT true;
