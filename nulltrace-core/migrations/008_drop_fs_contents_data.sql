-- Drop data column and make content_hash NOT NULL (run after data migration)
ALTER TABLE fs_contents DROP COLUMN IF EXISTS data;
ALTER TABLE fs_contents ALTER COLUMN content_hash SET NOT NULL;
