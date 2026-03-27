-- Hackerboard feed: user posts and replies with language; likes per player.
CREATE TABLE IF NOT EXISTS feed_posts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id    UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    body         TEXT NOT NULL,
    language     VARCHAR(16) NOT NULL CHECK (language IN ('en', 'pt-br')),
    reply_to_id  UUID REFERENCES feed_posts(id) ON DELETE CASCADE,
    post_type    VARCHAR(32) NOT NULL DEFAULT 'user' CHECK (post_type IN ('user', 'system', 'hacked', 'mission')),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_feed_posts_created_at ON feed_posts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_feed_posts_language ON feed_posts(language);
CREATE INDEX IF NOT EXISTS idx_feed_posts_author_id ON feed_posts(author_id);

CREATE TABLE IF NOT EXISTS feed_post_likes (
    post_id   UUID NOT NULL REFERENCES feed_posts(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (post_id, player_id)
);

CREATE INDEX IF NOT EXISTS idx_feed_post_likes_player_id ON feed_post_likes(player_id);
