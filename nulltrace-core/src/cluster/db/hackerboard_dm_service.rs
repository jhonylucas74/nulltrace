//! Persisted Hackerboard direct messages (Phase 2.1).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const MAX_BODY_CHARS: usize = 4000;

#[derive(Debug, Clone, FromRow)]
pub struct HackerboardDmThreadRow {
    pub peer_id: Uuid,
    pub peer_username: String,
    pub last_message_id: Uuid,
    pub last_body: String,
    pub last_created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct HackerboardDmMessageRow {
    pub id: Uuid,
    pub from_player_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

pub struct HackerboardDmService {
    pool: PgPool,
}

impl HackerboardDmService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Send a DM to `target_username`. Returns new message id or error string.
    pub async fn send_message(
        &self,
        from_id: Uuid,
        target_username: &str,
        body: &str,
    ) -> Result<Uuid, String> {
        let body = body.trim();
        if body.is_empty() {
            return Err("Message is empty".to_string());
        }
        if body.chars().count() > MAX_BODY_CHARS {
            return Err(format!("Message exceeds {} characters", MAX_BODY_CHARS));
        }

        let to_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM players WHERE username = $1",
        )
        .bind(target_username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Player not found".to_string())?;

        if to_id == from_id {
            return Err("Cannot message yourself".to_string());
        }

        let pair_blocked: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM player_blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            )
            "#,
        )
        .bind(from_id)
        .bind(to_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if pair_blocked {
            return Err("Cannot message this player".to_string());
        }

        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO hackerboard_dm_messages (id, from_player_id, to_player_id, body)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(from_id)
        .bind(to_id)
        .bind(body)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(id)
    }

    /// Latest thread per peer (other participant in a DM with `for_player_id`).
    pub async fn list_threads(
        &self,
        for_player_id: Uuid,
        limit: i64,
    ) -> Result<Vec<HackerboardDmThreadRow>, sqlx::Error> {
        let lim = limit.clamp(1, 100);
        sqlx::query_as::<_, HackerboardDmThreadRow>(
            r#"
            SELECT DISTINCT ON (t.peer_id)
                t.peer_id,
                p.username AS peer_username,
                t.id AS last_message_id,
                t.body AS last_body,
                t.created_at AS last_created_at
            FROM (
                SELECT
                    m.id,
                    m.body,
                    m.created_at,
                    CASE WHEN m.from_player_id = $1 THEN m.to_player_id ELSE m.from_player_id END AS peer_id
                FROM hackerboard_dm_messages m
                WHERE m.from_player_id = $1 OR m.to_player_id = $1
            ) t
            INNER JOIN players p ON p.id = t.peer_id
            WHERE NOT EXISTS (
                SELECT 1 FROM player_blocks blk
                WHERE (blk.blocker_id = $1 AND blk.blocked_id = t.peer_id)
                   OR (blk.blocker_id = t.peer_id AND blk.blocked_id = $1)
            )
            ORDER BY t.peer_id, t.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(for_player_id)
        .bind(lim)
        .fetch_all(&self.pool)
        .await
    }

    /// Messages between `for_player_id` and `peer_id`, oldest first (up to `limit`).
    /// If `before_message_id` is set, only messages strictly older than that message (by `created_at`, then `id`).
    pub async fn list_messages(
        &self,
        for_player_id: Uuid,
        peer_id: Uuid,
        before_message_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<HackerboardDmMessageRow>, String> {
        let lim = limit.clamp(1, 200);

        let pair_blocked: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM player_blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            )
            "#,
        )
        .bind(for_player_id)
        .bind(peer_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if pair_blocked {
            return Ok(vec![]);
        }

        // Ensure the two players have a DM relationship (any row between them).
        let any: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT m.id
            FROM hackerboard_dm_messages m
            WHERE (m.from_player_id = $1 AND m.to_player_id = $2)
               OR (m.from_player_id = $2 AND m.to_player_id = $1)
            LIMIT 1
            "#,
        )
        .bind(for_player_id)
        .bind(peer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if any.is_none() && before_message_id.is_none() {
            // No history yet — return empty (UI can still open thread to send first message).
            return Ok(vec![]);
        }

        let before_created: Option<chrono::DateTime<chrono::Utc>> = if let Some(bid) = before_message_id {
            let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
                r#"
                SELECT m.created_at
                FROM hackerboard_dm_messages m
                WHERE m.id = $1
                  AND ((m.from_player_id = $2 AND m.to_player_id = $3)
                    OR (m.from_player_id = $3 AND m.to_player_id = $2))
                "#,
            )
            .bind(bid)
            .bind(for_player_id)
            .bind(peer_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            Some(row.ok_or_else(|| "Invalid before_message_id".to_string())?.0)
        } else {
            None
        };

        let rows: Vec<HackerboardDmMessageRow> = if let Some(ts) = before_created {
            sqlx::query_as::<_, HackerboardDmMessageRow>(
                r#"
                SELECT m.id, m.from_player_id, m.body, m.created_at
                FROM hackerboard_dm_messages m
                WHERE (m.from_player_id = $1 AND m.to_player_id = $2)
                   OR (m.from_player_id = $2 AND m.to_player_id = $1)
                  AND (m.created_at, m.id) < ($3::timestamptz, $4::uuid)
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $5
                "#,
            )
            .bind(for_player_id)
            .bind(peer_id)
            .bind(ts)
            .bind(before_message_id.unwrap())
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query_as::<_, HackerboardDmMessageRow>(
                r#"
                SELECT m.id, m.from_player_id, m.body, m.created_at
                FROM hackerboard_dm_messages m
                WHERE (m.from_player_id = $1 AND m.to_player_id = $2)
                   OR (m.from_player_id = $2 AND m.to_player_id = $1)
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $3
                "#,
            )
            .bind(for_player_id)
            .bind(peer_id)
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        };

        let mut out = rows;
        out.reverse();
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::player_service::PlayerService;
    use crate::db::test_pool;

    #[tokio::test]
    async fn test_dm_send_and_list() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let dms = HackerboardDmService::new(pool);

        let a = format!("dm_a_{}", Uuid::new_v4());
        let b = format!("dm_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();

        let mid = dms
            .send_message(pa.id, &b, "hello")
            .await
            .expect("send");
        assert!(!mid.is_nil());

        let threads = dms.list_threads(pa.id, 50).await.unwrap();
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].peer_id, pb.id);
        assert_eq!(threads[0].last_body, "hello");

        let msgs = dms
            .list_messages(pa.id, pb.id, None, 50)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from_player_id, pa.id);
        assert_eq!(msgs[0].body, "hello");
    }

    #[tokio::test]
    async fn test_dm_self_rejected() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let dms = HackerboardDmService::new(pool);

        let a = format!("dm_self_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();

        let err = dms.send_message(pa.id, &a, "x").await.unwrap_err();
        assert!(err.contains("yourself") || err.contains("Cannot"));
    }

    #[tokio::test]
    async fn test_dm_unknown_user() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let dms = HackerboardDmService::new(pool);

        let a = format!("dm_u_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();

        let err = dms
            .send_message(pa.id, "definitely_no_such_user_xyz", "hi")
            .await
            .unwrap_err();
        assert!(err.contains("not found") || err.contains("Player"));
    }
}
