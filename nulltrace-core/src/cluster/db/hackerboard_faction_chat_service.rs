//! Persisted Hackerboard faction group chat (Phase 2.2).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

const MAX_BODY_CHARS: usize = 4000;

#[derive(Debug, Clone, FromRow)]
pub struct HackerboardFactionMessageRow {
    pub id: Uuid,
    pub from_player_id: Uuid,
    pub from_username: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

pub struct HackerboardFactionChatService {
    pool: PgPool,
}

impl HackerboardFactionChatService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn faction_id_for_player(&self, player_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        let opt = sqlx::query_scalar::<_, Option<Uuid>>("SELECT faction_id FROM players WHERE id = $1")
            .bind(player_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(opt.flatten())
    }

    pub async fn send_message(&self, from_id: Uuid, body: &str) -> Result<Uuid, String> {
        let body = body.trim();
        if body.is_empty() {
            return Err("Message is empty".to_string());
        }
        if body.chars().count() > MAX_BODY_CHARS {
            return Err(format!("Message exceeds {} characters", MAX_BODY_CHARS));
        }

        let faction_id = self
            .faction_id_for_player(from_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "You are not in a faction".to_string())?;

        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO hackerboard_faction_messages (id, faction_id, from_player_id, body)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(id)
        .bind(faction_id)
        .bind(from_id)
        .bind(body)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(id)
    }

    /// Messages in the caller's faction, oldest first within the returned window.
    /// Newest page: `before_message_id` None — returns the last `limit` messages (chronological order).
    /// Older: pass oldest loaded message id to fetch earlier messages.
    pub async fn list_messages(
        &self,
        for_player_id: Uuid,
        before_message_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<HackerboardFactionMessageRow>, String> {
        let lim = limit.clamp(1, 200);

        let faction_id = self
            .faction_id_for_player(for_player_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "You are not in a faction".to_string())?;

        let rows: Vec<HackerboardFactionMessageRow> = if let Some(bid) = before_message_id {
            let ts_row: Option<(DateTime<Utc>,)> = sqlx::query_as(
                r#"
                SELECT m.created_at
                FROM hackerboard_faction_messages m
                WHERE m.id = $1 AND m.faction_id = $2
                "#,
            )
            .bind(bid)
            .bind(faction_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
            let ts = ts_row.ok_or_else(|| "Invalid before_message_id".to_string())?.0;

            sqlx::query_as::<_, HackerboardFactionMessageRow>(
                r#"
                SELECT m.id, m.from_player_id, p.username AS from_username, m.body, m.created_at
                FROM hackerboard_faction_messages m
                INNER JOIN players p ON p.id = m.from_player_id
                WHERE m.faction_id = $1
                  AND (m.created_at, m.id) < ($2::timestamptz, $3::uuid)
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $4
                "#,
            )
            .bind(faction_id)
            .bind(ts)
            .bind(bid)
            .bind(lim)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query_as::<_, HackerboardFactionMessageRow>(
                r#"
                SELECT m.id, m.from_player_id, p.username AS from_username, m.body, m.created_at
                FROM hackerboard_faction_messages m
                INNER JOIN players p ON p.id = m.from_player_id
                WHERE m.faction_id = $1
                ORDER BY m.created_at DESC, m.id DESC
                LIMIT $2
                "#,
            )
            .bind(faction_id)
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
    use crate::db::faction_service::FactionService;
    use crate::db::player_service::PlayerService;
    use crate::db::test_pool;

    #[tokio::test]
    async fn test_faction_chat_send_list() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let chat = HackerboardFactionChatService::new(pool);

        let a = format!("fc_a_{}", Uuid::new_v4());
        let b = format!("fc_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();

        let fac = factions.create("FC Test", pa.id).await.unwrap();
        players
            .set_faction_id(pa.id, Some(fac.id))
            .await
            .unwrap();
        players
            .set_faction_id(pb.id, Some(fac.id))
            .await
            .unwrap();

        chat.send_message(pa.id, "one").await.unwrap();
        chat.send_message(pb.id, "two").await.unwrap();

        let msgs = chat.list_messages(pa.id, None, 50).await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].body, "one");
        assert_eq!(msgs[1].body, "two");
    }

    #[tokio::test]
    async fn test_faction_chat_not_in_faction() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let chat = HackerboardFactionChatService::new(pool);

        let a = format!("fc_nf_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();

        let err = chat.send_message(pa.id, "x").await.unwrap_err();
        assert!(err.contains("faction") || err.contains("not"));
    }
}
