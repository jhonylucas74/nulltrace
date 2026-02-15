-- Store user's preferred UI theme (e.g. githubdark, mocha).
ALTER TABLE players ADD COLUMN IF NOT EXISTS preferred_theme VARCHAR(32) DEFAULT 'githubdark';
