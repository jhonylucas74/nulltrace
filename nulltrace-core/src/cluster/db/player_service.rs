#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Default test player username (seed created on startup).
pub const SEED_USERNAME: &str = "Haru";
/// Default test player password (same as username for dev/test).
pub const SEED_PASSWORD: &str = "haru";

#[derive(Debug, Clone, FromRow)]
pub struct Player {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub points: i32,
    pub faction_id: Option<Uuid>,
    pub preferred_theme: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PlayerService {
    pool: PgPool,
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl PlayerService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_player(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Player, sqlx::Error> {
        let id = Uuid::new_v4();
        let password_hash = hash_password(password);
        let rec = sqlx::query_as::<_, Player>(
            r#"
            INSERT INTO players (id, username, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, username, password_hash, points, faction_id, preferred_theme, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(username)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_by_username(&self, username: &str) -> Result<Option<Player>, sqlx::Error> {
        let rec = sqlx::query_as::<_, Player>(
            r#"
            SELECT id, username, password_hash, points, faction_id, preferred_theme, created_at, updated_at
            FROM players WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Player>, sqlx::Error> {
        let rec = sqlx::query_as::<_, Player>(
            r#"
            SELECT id, username, password_hash, points, faction_id, preferred_theme, created_at, updated_at
            FROM players WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    /// Verify credentials; returns the player if valid.
    pub async fn verify_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<Player>, sqlx::Error> {
        let player = self.get_by_username(username).await?;
        Ok(match player {
            Some(p) if p.password_hash == hash_password(password) => Some(p),
            _ => None,
        })
    }

    /// Ensure the seed player (Haru) exists; create if not. Idempotent.
    pub async fn seed_haru(&self) -> Result<(), sqlx::Error> {
        if self.get_by_username(SEED_USERNAME).await?.is_some() {
            return Ok(());
        }
        self.create_player(SEED_USERNAME, SEED_PASSWORD).await?;
        Ok(())
    }

    /// Add delta to player points (can be negative). Returns new points.
    pub async fn add_points(&self, player_id: Uuid, delta: i32) -> Result<i32, sqlx::Error> {
        let rec = sqlx::query_as::<_, (i32,)>(
            r#"
            UPDATE players SET points = points + $2, updated_at = now()
            WHERE id = $1
            RETURNING points
            "#,
        )
        .bind(player_id)
        .bind(delta)
        .fetch_one(&self.pool)
        .await?;
        Ok(rec.0)
    }

    /// Set preferred UI theme for a player.
    pub async fn set_preferred_theme(
        &self,
        player_id: Uuid,
        theme: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE players SET preferred_theme = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(player_id)
        .bind(theme)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Set faction for a player (None to leave faction).
    pub async fn set_faction_id(
        &self,
        player_id: Uuid,
        faction_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE players SET faction_id = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(player_id)
        .bind(faction_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Ranking: all players ordered by points DESC with 1-based rank.
    pub async fn get_ranking(&self) -> Result<Vec<(u32, Uuid, String, i32, Option<Uuid>)>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (i64, Uuid, String, i32, Option<Uuid>)>(
            r#"
            SELECT ROW_NUMBER() OVER (ORDER BY points DESC) AS rank,
                   id, username, points, faction_id
            FROM players
            ORDER BY points DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(r, id, u, p, f)| (r as u32, id, u, p, f))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_pool;

    #[tokio::test]
    async fn test_create_player() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        let name = format!("testplayer_{}", Uuid::new_v4());

        let p = svc.create_player(&name, "testpass").await.unwrap();
        assert_eq!(p.username, name);
        assert!(!p.password_hash.is_empty());
        assert_ne!(p.password_hash, "testpass");

        let same = svc.get_by_username(&name).await.unwrap().unwrap();
        assert_eq!(same.id, p.id);
    }

    #[tokio::test]
    async fn test_get_by_username_nonexistent() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        assert!(svc.get_by_username("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        let name = format!("byid_{}", Uuid::new_v4());
        let p = svc.create_player(&name, "pw").await.unwrap();
        let found = svc.get_by_id(p.id).await.unwrap().unwrap();
        assert_eq!(found.username, name);
        assert!(svc.get_by_id(Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_verify_password_correct() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        let name = format!("validuser_{}", Uuid::new_v4());
        svc.create_player(&name, "secret").await.unwrap();

        let player = svc.verify_password(&name, "secret").await.unwrap();
        assert!(player.is_some());
        assert_eq!(player.unwrap().username, name);
    }

    #[tokio::test]
    async fn test_verify_password_wrong() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        let name = format!("wrong_pass_{}", Uuid::new_v4());
        svc.create_player(&name, "right").await.unwrap();

        assert!(svc.verify_password(&name, "wrong").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_verify_password_nonexistent_username() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);
        assert!(svc.verify_password("nobody", "any").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_seed_haru_creates_once() {
        let pool = test_pool().await;
        let svc = PlayerService::new(pool);

        svc.seed_haru().await.unwrap();
        let haru1 = svc.get_by_username(SEED_USERNAME).await.unwrap().unwrap();

        svc.seed_haru().await.unwrap();
        let haru2 = svc.get_by_username(SEED_USERNAME).await.unwrap().unwrap();

        assert_eq!(haru1.id, haru2.id, "seed_haru should be idempotent");
        assert!(svc.verify_password(SEED_USERNAME, SEED_PASSWORD).await.unwrap().is_some());
    }
}
