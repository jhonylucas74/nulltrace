//! Faction kick (creator) and faction-scoped ban from rejoining.

use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct FactionBannedMemberRow {
    pub player_id: Uuid,
    pub username: String,
}

pub struct FactionMemberService {
    pool: PgPool,
}

impl FactionMemberService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creator removes another member from the faction; optionally bans rejoin via invites.
    pub async fn kick_member(
        &self,
        creator_id: Uuid,
        target_username: &str,
        ban_from_rejoin: bool,
    ) -> Result<(), String> {
        let target_username = target_username.trim();
        if target_username.is_empty() {
            return Err("Username is required".to_string());
        }

        let creator = self
            .player_row(creator_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Player not found".to_string())?;

        let faction_id = creator
            .faction_id
            .ok_or_else(|| "You are not in a faction".to_string())?;

        let fac_creator: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT creator_id FROM factions WHERE id = $1",
        )
        .bind(faction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let fac_creator = fac_creator
            .flatten()
            .ok_or_else(|| "Faction has no creator".to_string())?;

        if fac_creator != creator_id {
            return Err("Only the faction creator can kick members".to_string());
        }

        let target = self
            .player_by_username(target_username)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Player not found".to_string())?;

        if target.id == creator_id {
            return Err("Cannot kick the faction leader".to_string());
        }

        if target.faction_id != Some(faction_id) {
            return Err("That player is not in your faction".to_string());
        }

        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let res = sqlx::query(
            r#"
            UPDATE players SET faction_id = NULL, updated_at = now()
            WHERE id = $1 AND faction_id = $2
            "#,
        )
        .bind(target.id)
        .bind(faction_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() != 1 {
            return Err("That player is not in your faction".to_string());
        }

        sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'cancelled', updated_at = now()
            WHERE from_player_id = $1 AND status = 'pending'
            "#,
        )
        .bind(target.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'cancelled', updated_at = now()
            WHERE faction_id = $1 AND to_player_id = $2 AND status = 'pending'
            "#,
        )
        .bind(faction_id)
        .bind(target.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if ban_from_rejoin {
            sqlx::query(
                r#"
                INSERT INTO faction_member_bans (faction_id, banned_player_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(faction_id)
            .bind(target.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        }

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Creator removes a faction-scoped ban so the player can be invited again.
    pub async fn unban_member(&self, creator_id: Uuid, target_username: &str) -> Result<(), String> {
        let target_username = target_username.trim();
        if target_username.is_empty() {
            return Err("Username is required".to_string());
        }

        let creator = self
            .player_row(creator_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Player not found".to_string())?;

        let faction_id = creator
            .faction_id
            .ok_or_else(|| "You are not in a faction".to_string())?;

        let fac_creator: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT creator_id FROM factions WHERE id = $1",
        )
        .bind(faction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let fac_creator = fac_creator
            .flatten()
            .ok_or_else(|| "Faction has no creator".to_string())?;

        if fac_creator != creator_id {
            return Err("Only the faction creator can unban players".to_string());
        }

        let target = self
            .player_by_username(target_username)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Player not found".to_string())?;

        let res = sqlx::query(
            r#"
            DELETE FROM faction_member_bans
            WHERE faction_id = $1 AND banned_player_id = $2
            "#,
        )
        .bind(faction_id)
        .bind(target.id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err("That player is not banned from this faction".to_string());
        }

        Ok(())
    }

    pub async fn list_banned_members(
        &self,
        creator_id: Uuid,
    ) -> Result<Vec<FactionBannedMemberRow>, String> {
        let creator = self
            .player_row(creator_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Player not found".to_string())?;

        let faction_id = creator
            .faction_id
            .ok_or_else(|| "You are not in a faction".to_string())?;

        let fac_creator: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT creator_id FROM factions WHERE id = $1",
        )
        .bind(faction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        let fac_creator = fac_creator
            .flatten()
            .ok_or_else(|| "Faction has no creator".to_string())?;

        if fac_creator != creator_id {
            return Err("Only the faction creator can list banned players".to_string());
        }

        let rows = sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT p.id, p.username
            FROM faction_member_bans b
            INNER JOIN players p ON p.id = b.banned_player_id
            WHERE b.faction_id = $1
            ORDER BY p.username ASC
            "#,
        )
        .bind(faction_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .map(|(player_id, username)| FactionBannedMemberRow {
                player_id,
                username,
            })
            .collect())
    }

    async fn player_row(&self, id: Uuid) -> Result<Option<PlayerFactionRow>, sqlx::Error> {
        sqlx::query_as::<_, PlayerFactionRow>(
            r#"
            SELECT id, username, faction_id FROM players WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn player_by_username(
        &self,
        username: &str,
    ) -> Result<Option<PlayerFactionRow>, sqlx::Error> {
        sqlx::query_as::<_, PlayerFactionRow>(
            r#"
            SELECT id, username, faction_id FROM players WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PlayerFactionRow {
    id: Uuid,
    #[allow(dead_code)]
    username: String,
    faction_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::faction_invite_service::FactionInviteService;
    use crate::db::faction_service::FactionService;
    use crate::db::player_service::PlayerService;
    use crate::db::{self};

    #[tokio::test]
    async fn kick_ban_blocks_invite_until_unban() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool.clone());
        let members = FactionMemberService::new(pool.clone());

        let pa = players.create_player("KickBanA", "x").await.unwrap();
        let pb = players.create_player("KickBanB", "x").await.unwrap();
        let pc = players.create_player("KickBanC", "x").await.unwrap();

        let fac = factions.create("KickBanFac", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();
        players.set_faction_id(pb.id, Some(fac.id)).await.unwrap();

        members
            .kick_member(pa.id, "KickBanB", true)
            .await
            .expect("kick");

        let pb_after = players.get_by_id(pb.id).await.unwrap().unwrap();
        assert!(pb_after.faction_id.is_none());

        let err = invites
            .create_invite(fac.id, pa.id, pc.id)
            .await
            .err();
        assert!(err.is_none(), "C should be invitable");

        let err = invites
            .create_invite(fac.id, pa.id, pb.id)
            .await
            .err()
            .expect("banned B should not get invite");
        assert!(
            err.contains("banned"),
            "unexpected: {}",
            err
        );

        members
            .unban_member(pa.id, "KickBanB")
            .await
            .expect("unban");

        let id = invites
            .create_invite(fac.id, pa.id, pb.id)
            .await
            .expect("invite after unban");
        assert!(!id.is_nil());
    }

    #[tokio::test]
    async fn non_creator_cannot_kick() {
        let pool = db::test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let members = FactionMemberService::new(pool.clone());

        let pa = players.create_player("KickNonA", "x").await.unwrap();
        let pb = players.create_player("KickNonB", "x").await.unwrap();

        let fac = factions.create("KickNonFac", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();
        players.set_faction_id(pb.id, Some(fac.id)).await.unwrap();

        let err = members
            .kick_member(pb.id, "KickNonA", false)
            .await
            .err()
            .unwrap();
        assert!(err.contains("creator"), "{}", err);
    }
}
