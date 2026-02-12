#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Faction {
    pub id: Uuid,
    pub name: String,
    pub creator_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct FactionService {
    pool: PgPool,
}

impl FactionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, creator_id: Uuid) -> Result<Faction, sqlx::Error> {
        let rec = sqlx::query_as::<_, Faction>(
            r#"
            INSERT INTO factions (id, name, creator_id)
            VALUES (gen_random_uuid(), $1, $2)
            RETURNING id, name, creator_id, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(creator_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Faction>, sqlx::Error> {
        let rec = sqlx::query_as::<_, Faction>(
            r#"
            SELECT id, name, creator_id, created_at, updated_at
            FROM factions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rec)
    }

    /// Member IDs for a faction (players with faction_id = id).
    pub async fn list_member_ids(&self, faction_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT id FROM players WHERE faction_id = $1
            "#,
        )
        .bind(faction_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Total points of all members of a faction.
    pub async fn total_points(&self, faction_id: Uuid) -> Result<i64, sqlx::Error> {
        let row = sqlx::query_as::<_, (Option<i64>,)>(
            r#"
            SELECT COALESCE(SUM(points), 0) FROM players WHERE faction_id = $1
            "#,
        )
        .bind(faction_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0.unwrap_or(0))
    }
}
