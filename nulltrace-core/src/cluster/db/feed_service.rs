//! Hackerboard feed: posts with language and per-player likes.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

const MAX_BODY_LEN: usize = 4000;

pub const FEED_LANG_EN: &str = "en";
pub const FEED_LANG_PT_BR: &str = "pt-br";

#[derive(Debug, Clone)]
pub struct FeedPostRow {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub body: String,
    pub language: String,
    pub reply_to_id: Option<Uuid>,
    pub post_type: String,
    pub created_at: DateTime<Utc>,
    pub like_count: i32,
    pub liked_by_me: bool,
}

pub struct FeedService {
    pool: PgPool,
}

impl FeedService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn normalize_language(s: &str) -> Option<&'static str> {
        let t = s.trim();
        if t == FEED_LANG_EN {
            Some(FEED_LANG_EN)
        } else if t == FEED_LANG_PT_BR {
            Some(FEED_LANG_PT_BR)
        } else {
            None
        }
    }

    /// `language_filter` None = all posts; Some("en") / Some("pt-br") = filter.
    /// `before_post_id` None = first page; Some(id) = posts strictly older than that row (keyset).
    pub async fn list_posts(
        &self,
        language_filter: Option<&str>,
        limit: i32,
        current_player_id: Uuid,
        before_post_id: Option<Uuid>,
    ) -> Result<Vec<FeedPostRow>, sqlx::Error> {
        let lim = limit.clamp(1, 100) as i64;

        let rows = match (language_filter, before_post_id) {
            (Some(lang), None) => {
                sqlx::query_as::<_, FeedPostRowSql>(
                    r#"
                SELECT
                    fp.id,
                    fp.author_id,
                    p.username AS author_username,
                    fp.body,
                    fp.language,
                    fp.reply_to_id,
                    fp.post_type,
                    fp.created_at,
                    COALESCE(lc.cnt, 0)::int AS like_count,
                    COALESCE(ml.liked, false) AS liked_by_me
                FROM feed_posts fp
                INNER JOIN players p ON p.id = fp.author_id
                LEFT JOIN (
                    SELECT post_id, COUNT(*)::bigint AS cnt
                    FROM feed_post_likes
                    GROUP BY post_id
                ) lc ON lc.post_id = fp.id
                LEFT JOIN (
                    SELECT post_id, true AS liked
                    FROM feed_post_likes
                    WHERE player_id = $2
                ) ml ON ml.post_id = fp.id
                WHERE (
                    (fp.reply_to_id IS NULL AND fp.language = $1)
                    OR (
                        fp.reply_to_id IS NOT NULL
                        AND EXISTS (
                            SELECT 1 FROM feed_posts parent
                            WHERE parent.id = fp.reply_to_id AND parent.language = $1
                        )
                    )
                )
                AND NOT EXISTS (
                    SELECT 1 FROM player_blocks blk
                    WHERE (blk.blocker_id = $2 AND blk.blocked_id = fp.author_id)
                       OR (blk.blocker_id = fp.author_id AND blk.blocked_id = $2)
                )
                ORDER BY fp.created_at DESC
                LIMIT $3
                "#,
                )
                .bind(lang)
                .bind(current_player_id)
                .bind(lim)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(lang), Some(cursor)) => {
                sqlx::query_as::<_, FeedPostRowSql>(
                    r#"
                SELECT
                    fp.id,
                    fp.author_id,
                    p.username AS author_username,
                    fp.body,
                    fp.language,
                    fp.reply_to_id,
                    fp.post_type,
                    fp.created_at,
                    COALESCE(lc.cnt, 0)::int AS like_count,
                    COALESCE(ml.liked, false) AS liked_by_me
                FROM feed_posts fp
                INNER JOIN players p ON p.id = fp.author_id
                LEFT JOIN (
                    SELECT post_id, COUNT(*)::bigint AS cnt
                    FROM feed_post_likes
                    GROUP BY post_id
                ) lc ON lc.post_id = fp.id
                LEFT JOIN (
                    SELECT post_id, true AS liked
                    FROM feed_post_likes
                    WHERE player_id = $2
                ) ml ON ml.post_id = fp.id
                WHERE (
                    (fp.reply_to_id IS NULL AND fp.language = $1)
                    OR (
                        fp.reply_to_id IS NOT NULL
                        AND EXISTS (
                            SELECT 1 FROM feed_posts parent
                            WHERE parent.id = fp.reply_to_id AND parent.language = $1
                        )
                    )
                )
                AND NOT EXISTS (
                    SELECT 1 FROM player_blocks blk
                    WHERE (blk.blocker_id = $2 AND blk.blocked_id = fp.author_id)
                       OR (blk.blocker_id = fp.author_id AND blk.blocked_id = $2)
                )
                  AND (fp.created_at, fp.id) < (
                      SELECT created_at, id FROM feed_posts WHERE id = $4
                  )
                ORDER BY fp.created_at DESC
                LIMIT $3
                "#,
                )
                .bind(lang)
                .bind(current_player_id)
                .bind(lim)
                .bind(cursor)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, FeedPostRowSql>(
                    r#"
                SELECT
                    fp.id,
                    fp.author_id,
                    p.username AS author_username,
                    fp.body,
                    fp.language,
                    fp.reply_to_id,
                    fp.post_type,
                    fp.created_at,
                    COALESCE(lc.cnt, 0)::int AS like_count,
                    COALESCE(ml.liked, false) AS liked_by_me
                FROM feed_posts fp
                INNER JOIN players p ON p.id = fp.author_id
                LEFT JOIN (
                    SELECT post_id, COUNT(*)::bigint AS cnt
                    FROM feed_post_likes
                    GROUP BY post_id
                ) lc ON lc.post_id = fp.id
                LEFT JOIN (
                    SELECT post_id, true AS liked
                    FROM feed_post_likes
                    WHERE player_id = $1
                ) ml ON ml.post_id = fp.id
                WHERE NOT EXISTS (
                    SELECT 1 FROM player_blocks blk
                    WHERE (blk.blocker_id = $1 AND blk.blocked_id = fp.author_id)
                       OR (blk.blocker_id = fp.author_id AND blk.blocked_id = $1)
                )
                ORDER BY fp.created_at DESC
                LIMIT $2
                "#,
                )
                .bind(current_player_id)
                .bind(lim)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(cursor)) => {
                sqlx::query_as::<_, FeedPostRowSql>(
                    r#"
                SELECT
                    fp.id,
                    fp.author_id,
                    p.username AS author_username,
                    fp.body,
                    fp.language,
                    fp.reply_to_id,
                    fp.post_type,
                    fp.created_at,
                    COALESCE(lc.cnt, 0)::int AS like_count,
                    COALESCE(ml.liked, false) AS liked_by_me
                FROM feed_posts fp
                INNER JOIN players p ON p.id = fp.author_id
                LEFT JOIN (
                    SELECT post_id, COUNT(*)::bigint AS cnt
                    FROM feed_post_likes
                    GROUP BY post_id
                ) lc ON lc.post_id = fp.id
                LEFT JOIN (
                    SELECT post_id, true AS liked
                    FROM feed_post_likes
                    WHERE player_id = $1
                ) ml ON ml.post_id = fp.id
                WHERE NOT EXISTS (
                    SELECT 1 FROM player_blocks blk
                    WHERE (blk.blocker_id = $1 AND blk.blocked_id = fp.author_id)
                       OR (blk.blocker_id = fp.author_id AND blk.blocked_id = $1)
                )
                AND (fp.created_at, fp.id) < (
                    SELECT created_at, id FROM feed_posts WHERE id = $3
                )
                ORDER BY fp.created_at DESC
                LIMIT $2
                "#,
                )
                .bind(current_player_id)
                .bind(lim)
                .bind(cursor)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(rows.into_iter().map(|r| r.into_row()).collect())
    }

    pub async fn create_post(
        &self,
        author_id: Uuid,
        body: &str,
        language: &str,
        reply_to_post_id: Option<Uuid>,
    ) -> Result<FeedPostRow, String> {
        let body = body.trim();
        if body.is_empty() {
            return Err("Body is required".to_string());
        }
        if body.chars().count() > MAX_BODY_LEN {
            return Err(format!("Body exceeds {} characters", MAX_BODY_LEN));
        }
        let lang = Self::normalize_language(language).ok_or_else(|| "Invalid language".to_string())?;

        if let Some(parent_id) = reply_to_post_id {
            let exists: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM feed_posts WHERE id = $1",
            )
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            if exists.is_none() {
                return Err("Reply target not found".to_string());
            }
        }

        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO feed_posts (author_id, body, language, reply_to_id, post_type)
            VALUES ($1, $2, $3, $4, 'user')
            RETURNING id
            "#,
        )
        .bind(author_id)
        .bind(body)
        .bind(lang)
        .bind(reply_to_post_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        self.fetch_post_row(id, author_id).await
    }

    async fn fetch_post_row(&self, id: Uuid, current_player_id: Uuid) -> Result<FeedPostRow, String> {
        let row = sqlx::query_as::<_, FeedPostRowSql>(
            r#"
            SELECT
                fp.id,
                fp.author_id,
                p.username AS author_username,
                fp.body,
                fp.language,
                fp.reply_to_id,
                fp.post_type,
                fp.created_at,
                COALESCE(lc.cnt, 0)::int AS like_count,
                COALESCE(ml.liked, false) AS liked_by_me
            FROM feed_posts fp
            INNER JOIN players p ON p.id = fp.author_id
            LEFT JOIN (
                SELECT post_id, COUNT(*)::bigint AS cnt
                FROM feed_post_likes
                GROUP BY post_id
            ) lc ON lc.post_id = fp.id
            LEFT JOIN (
                SELECT post_id, true AS liked
                FROM feed_post_likes
                WHERE player_id = $2
            ) ml ON ml.post_id = fp.id
            WHERE fp.id = $1
            AND NOT EXISTS (
                SELECT 1 FROM player_blocks blk
                WHERE (blk.blocker_id = $2 AND blk.blocked_id = fp.author_id)
                   OR (blk.blocker_id = fp.author_id AND blk.blocked_id = $2)
            )
            "#,
        )
        .bind(id)
        .bind(current_player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        row.map(|r| r.into_row())
            .ok_or_else(|| "Post not found after insert".to_string())
    }

    /// Returns (liked, like_count).
    pub async fn toggle_like(&self, post_id: Uuid, player_id: Uuid) -> Result<(bool, i32), String> {
        let post_exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM feed_posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        if post_exists.is_none() {
            return Err("Post not found".to_string());
        }

        let had_like: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM feed_post_likes WHERE post_id = $1 AND player_id = $2)",
        )
        .bind(post_id)
        .bind(player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if had_like {
            sqlx::query("DELETE FROM feed_post_likes WHERE post_id = $1 AND player_id = $2")
                .bind(post_id)
                .bind(player_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            sqlx::query("INSERT INTO feed_post_likes (post_id, player_id) VALUES ($1, $2)")
                .bind(post_id)
                .bind(player_id)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint FROM feed_post_likes WHERE post_id = $1",
        )
        .bind(post_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let liked = !had_like;
        Ok((liked, count.0.clamp(0, i64::from(i32::MAX)) as i32))
    }
}

#[derive(sqlx::FromRow)]
struct FeedPostRowSql {
    id: Uuid,
    author_id: Uuid,
    author_username: String,
    body: String,
    language: String,
    reply_to_id: Option<Uuid>,
    post_type: String,
    created_at: DateTime<Utc>,
    like_count: i32,
    liked_by_me: bool,
}

impl FeedPostRowSql {
    fn into_row(self) -> FeedPostRow {
        FeedPostRow {
            id: self.id,
            author_id: self.author_id,
            author_username: self.author_username,
            body: self.body,
            language: self.language,
            reply_to_id: self.reply_to_id,
            post_type: self.post_type,
            created_at: self.created_at,
            like_count: self.like_count,
            liked_by_me: self.liked_by_me,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::player_service::PlayerService;
    use super::super::test_pool;

    #[tokio::test]
    async fn test_create_post_en_and_list_all() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_user_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        let row = feed
            .create_post(p.id, "Hello world", FEED_LANG_EN, None)
            .await
            .expect("create_post");

        assert_eq!(row.body, "Hello world");
        assert_eq!(row.language, FEED_LANG_EN);
        assert!(row.reply_to_id.is_none());
        assert_eq!(row.author_id, p.id);
        assert_eq!(row.author_username, name);

        let listed = feed.list_posts(None, 50, p.id, None).await.unwrap();
        assert!(listed.iter().any(|r| r.id == row.id));
    }

    #[tokio::test]
    async fn test_create_post_rejects_invalid_language() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_bad_lang_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        let err = feed
            .create_post(p.id, "x", "fr", None)
            .await
            .expect_err("invalid language");
        assert!(err.contains("Invalid language"));
    }

    #[tokio::test]
    async fn test_list_posts_filters_by_language() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_filter_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        feed.create_post(p.id, "English only", FEED_LANG_EN, None)
            .await
            .unwrap();
        feed.create_post(p.id, "So português", FEED_LANG_PT_BR, None)
            .await
            .unwrap();

        let all = feed.list_posts(None, 50, p.id, None).await.unwrap();
        let en_count = all.iter().filter(|r| r.language == FEED_LANG_EN).count();
        let pt_count = all.iter().filter(|r| r.language == FEED_LANG_PT_BR).count();
        assert!(en_count >= 1);
        assert!(pt_count >= 1);

        let en_only = feed
            .list_posts(Some(FEED_LANG_EN), 50, p.id, None)
            .await
            .unwrap();
        assert!(en_only
            .iter()
            .filter(|r| r.reply_to_id.is_none())
            .all(|r| r.language == FEED_LANG_EN));
        assert!(!en_only.iter().any(|r| r.body == "So português"));

        let pt_only = feed
            .list_posts(Some(FEED_LANG_PT_BR), 50, p.id, None)
            .await
            .unwrap();
        assert!(pt_only
            .iter()
            .filter(|r| r.reply_to_id.is_none())
            .all(|r| r.language == FEED_LANG_PT_BR));
        assert!(!pt_only.iter().any(|r| r.body == "English only"));
    }

    #[tokio::test]
    async fn test_list_posts_language_filter_includes_replies_by_parent_language() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_reply_lang_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        let root = feed
            .create_post(p.id, "root en", FEED_LANG_EN, None)
            .await
            .unwrap();
        feed.create_post(p.id, "reply in pt", FEED_LANG_PT_BR, Some(root.id))
            .await
            .unwrap();

        let en_feed = feed
            .list_posts(Some(FEED_LANG_EN), 50, p.id, None)
            .await
            .unwrap();
        assert!(
            en_feed.iter().any(|r| r.body == "reply in pt"),
            "reply should appear when parent matches filter language"
        );

        let pt_feed = feed
            .list_posts(Some(FEED_LANG_PT_BR), 50, p.id, None)
            .await
            .unwrap();
        assert!(
            !pt_feed.iter().any(|r| r.body == "reply in pt"),
            "reply should not appear when parent language does not match filter"
        );
    }

    #[tokio::test]
    async fn test_reply_requires_existing_parent() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_reply_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        let bad_parent = Uuid::new_v4();
        let err = feed
            .create_post(p.id, "orphan reply", FEED_LANG_EN, Some(bad_parent))
            .await
            .expect_err("missing parent");
        assert!(err.contains("Reply target not found"));

        let root = feed
            .create_post(p.id, "root", FEED_LANG_EN, None)
            .await
            .unwrap();
        let reply = feed
            .create_post(p.id, "nested", FEED_LANG_PT_BR, Some(root.id))
            .await
            .expect("reply");
        assert_eq!(reply.reply_to_id, Some(root.id));
    }

    #[tokio::test]
    async fn test_toggle_like_idempotent_count() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let a = format!("feed_like_a_{}", Uuid::new_v4());
        let b = format!("feed_like_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();

        let post = feed
            .create_post(pa.id, "like me", FEED_LANG_EN, None)
            .await
            .unwrap();

        let (liked1, count1) = feed.toggle_like(post.id, pb.id).await.unwrap();
        assert!(liked1);
        assert_eq!(count1, 1);

        let listed = feed.list_posts(None, 10, pb.id, None).await.unwrap();
        let row = listed.iter().find(|r| r.id == post.id).expect("post in list");
        assert!(row.liked_by_me);
        assert_eq!(row.like_count, 1);

        let (liked2, count2) = feed.toggle_like(post.id, pb.id).await.unwrap();
        assert!(!liked2);
        assert_eq!(count2, 0);

        let listed2 = feed.list_posts(None, 10, pb.id, None).await.unwrap();
        let row2 = listed2.iter().find(|r| r.id == post.id).expect("post in list");
        assert!(!row2.liked_by_me);
        assert_eq!(row2.like_count, 0);
    }

    #[tokio::test]
    async fn test_list_posts_keyset_pagination() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let feed = FeedService::new(pool);
        let name = format!("feed_page_{}", Uuid::new_v4());
        let p = players.create_player(&name, "pw").await.unwrap();

        for i in 0..3 {
            feed.create_post(p.id, &format!("post {i}"), FEED_LANG_EN, None)
                .await
                .unwrap();
        }

        let page1 = feed.list_posts(None, 2, p.id, None).await.unwrap();
        assert_eq!(page1.len(), 2);
        let p1_last = page1.last().unwrap();

        let page2 = feed
            .list_posts(None, 2, p.id, Some(p1_last.id))
            .await
            .unwrap();
        assert!(!page2.is_empty(), "second page should return older rows");
        let p1_set: std::collections::HashSet<_> = page1.iter().map(|r| r.id).collect();
        for r in &page2 {
            assert!(!p1_set.contains(&r.id), "no overlap between pages");
            assert!(
                (r.created_at, r.id) < (p1_last.created_at, p1_last.id),
                "keyset: page2 rows strictly older than cursor"
            );
        }
    }
}
