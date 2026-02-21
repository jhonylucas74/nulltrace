-- Store CC for display in email detail (optional; used on main recipient and sent copy).
ALTER TABLE emails ADD COLUMN IF NOT EXISTS cc_address TEXT;
