//! Hackerboard player blocks: hide users from each other in feed, ranking, and DMs.

use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BlockedPlayerRow {
    pub blocked_id: Uuid,
    pub username: String,
}

pub struct PlayerBlockService {
    pool: PgPool,
}

impl PlayerBlockService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// True if either direction exists between a and b.
    pub async fn is_pair_blocked(&self, a: Uuid, b: Uuid) -> Result<bool, sqlx::Error> {
        if a == b {
            return Ok(false);
        }
        let v: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM player_blocks
                WHERE (blocker_id = $1 AND blocked_id = $2)
                   OR (blocker_id = $2 AND blocked_id = $1)
            )
            "#,
        )
        .bind(a)
        .bind(b)
        .fetch_one(&self.pool)
        .await?;
        Ok(v)
    }

    /// Player ids the viewer must not see (either direction).
    pub async fn list_hidden_player_ids_for_viewer(&self, viewer_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT blocked_id FROM player_blocks WHERE blocker_id = $1
            UNION
            SELECT blocker_id FROM player_blocks WHERE blocked_id = $1
            "#,
        )
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Players blocked by `blocker_id` (for settings / unblock UI).
    pub async fn list_blocked_by_blocker(
        &self,
        blocker_id: Uuid,
    ) -> Result<Vec<BlockedPlayerRow>, sqlx::Error> {
        sqlx::query_as::<_, BlockedPlayerRow>(
            r#"
            SELECT pb.blocked_id, p.username
            FROM player_blocks pb
            INNER JOIN players p ON p.id = pb.blocked_id
            WHERE pb.blocker_id = $1
            ORDER BY pb.created_at DESC
            "#,
        )
        .bind(blocker_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn block(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<(), String> {
        if blocker_id == blocked_id {
            return Err("Cannot block yourself".to_string());
        }
        let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM players WHERE id = $1)")
            .bind(blocked_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err("Player not found".to_string());
        }
        sqlx::query(
            r#"
            INSERT INTO player_blocks (blocker_id, blocked_id)
            VALUES ($1, $2)
            ON CONFLICT (blocker_id, blocked_id) DO NOTHING
            "#,
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn unblock(&self, blocker_id: Uuid, blocked_id: Uuid) -> Result<(), String> {
        let res = sqlx::query(
            "DELETE FROM player_blocks WHERE blocker_id = $1 AND blocked_id = $2",
        )
        .bind(blocker_id)
        .bind(blocked_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        if res.rows_affected() == 0 {
            return Err("Not blocked".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::player_service::PlayerService;
    use crate::db::test_pool;

    #[tokio::test]
    async fn test_block_unblock_list() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let blocks = PlayerBlockService::new(pool);

        let a = format!("blk_a_{}", Uuid::new_v4());
        let b = format!("blk_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();

        assert!(!blocks.is_pair_blocked(pa.id, pb.id).await.unwrap());
        blocks.block(pa.id, pb.id).await.unwrap();
        assert!(blocks.is_pair_blocked(pa.id, pb.id).await.unwrap());

        let hidden = blocks.list_hidden_player_ids_for_viewer(pa.id).await.unwrap();
        assert!(hidden.contains(&pb.id));
        let hidden_b = blocks.list_hidden_player_ids_for_viewer(pb.id).await.unwrap();
        assert!(hidden_b.contains(&pa.id));

        let listed = blocks.list_blocked_by_blocker(pa.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].blocked_id, pb.id);

        blocks.unblock(pa.id, pb.id).await.unwrap();
        assert!(!blocks.is_pair_blocked(pa.id, pb.id).await.unwrap());
    }
}
