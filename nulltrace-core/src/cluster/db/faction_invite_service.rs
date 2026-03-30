//! Persisted faction invites (send / list / accept / decline).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Row for listing incoming pending invites (with faction name and inviter username).
#[derive(Debug, Clone, FromRow)]
pub struct FactionInviteListRow {
    pub id: Uuid,
    pub faction_id: Uuid,
    pub faction_name: String,
    pub from_username: String,
    pub created_at: DateTime<Utc>,
}

/// Pending invite sent from a faction (outbound list for members).
#[derive(Debug, Clone, FromRow)]
pub struct FactionInviteOutgoingRow {
    pub id: Uuid,
    pub to_username: String,
    pub from_username: String,
    pub from_player_id: Uuid,
    pub created_at: DateTime<Utc>,
}

pub struct FactionInviteService {
    pool: PgPool,
}

impl FactionInviteService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Invalidate pending invites sent by this player (e.g. after leaving a faction).
    pub async fn cancel_pending_sent_by(&self, from_player_id: Uuid) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'cancelled', updated_at = now()
            WHERE from_player_id = $1 AND status = 'pending'
            "#,
        )
        .bind(from_player_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Create a pending invite. Returns error message on business rule failure.
    pub async fn create_invite(
        &self,
        faction_id: Uuid,
        from_player_id: Uuid,
        to_player_id: Uuid,
    ) -> Result<Uuid, String> {
        if from_player_id == to_player_id {
            return Err("Cannot invite yourself".to_string());
        }

        let from = sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT faction_id FROM players WHERE id = $1",
        )
        .bind(from_player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Inviter not found".to_string())?;

        let from_faction = from.0.ok_or_else(|| "You are not in a faction".to_string())?;
        if from_faction != faction_id {
            return Err("You are not a member of that faction".to_string());
        }

        let fac_meta: Option<(Option<Uuid>, bool)> = sqlx::query_as(
            "SELECT creator_id, allow_member_invites FROM factions WHERE id = $1",
        )
        .bind(faction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let (creator_id, allow_member_invites) =
            fac_meta.ok_or_else(|| "Faction not found".to_string())?;
        if !allow_member_invites {
            let cr = creator_id.ok_or_else(|| "Faction has no creator".to_string())?;
            if from_player_id != cr {
                return Err("Only the faction creator can send invites".to_string());
            }
        }

        let to = sqlx::query_as::<_, (Option<Uuid>,)>(
            "SELECT faction_id FROM players WHERE id = $1",
        )
        .bind(to_player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Player not found".to_string())?;

        if to.0.is_some() {
            return Err("Player is already in a faction".to_string());
        }

        let exists = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM faction_invites
                WHERE faction_id = $1 AND to_player_id = $2 AND status = 'pending'
            )
            "#,
        )
        .bind(faction_id)
        .bind(to_player_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if exists {
            return Err("An invite to this player for this faction is already pending".to_string());
        }

        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO faction_invites (id, faction_id, from_player_id, to_player_id, status)
            VALUES ($1, $2, $3, $4, 'pending')
            "#,
        )
        .bind(id)
        .bind(faction_id)
        .bind(from_player_id)
        .bind(to_player_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(dbe) = &e {
                if dbe.code().as_deref() == Some("23505") {
                    return "An invite to this player for this faction is already pending".to_string();
                }
            }
            e.to_string()
        })?;

        Ok(id)
    }

    pub async fn list_pending_for_player(
        &self,
        to_player_id: Uuid,
    ) -> Result<Vec<FactionInviteListRow>, sqlx::Error> {
        sqlx::query_as::<_, FactionInviteListRow>(
            r#"
            SELECT
                fi.id,
                fi.faction_id,
                f.name AS faction_name,
                p_from.username AS from_username,
                fi.created_at
            FROM faction_invites fi
            INNER JOIN factions f ON f.id = fi.faction_id
            INNER JOIN players p_from ON p_from.id = fi.from_player_id
            WHERE fi.to_player_id = $1 AND fi.status = 'pending'
            ORDER BY fi.created_at DESC
            "#,
        )
        .bind(to_player_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn accept_invite(&self, invite_id: Uuid, to_player_id: Uuid) -> Result<(), String> {
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;

        let row = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            r#"
            SELECT faction_id, to_player_id, status
            FROM faction_invites
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(invite_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let (faction_id, invite_to, status) =
            row.ok_or_else(|| "Invite not found".to_string())?;

        if invite_to != to_player_id {
            return Err("Invite not for you".to_string());
        }
        if status != "pending" {
            return Err("Invite is no longer pending".to_string());
        }

        let current_faction = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT faction_id FROM players WHERE id = $1 FOR UPDATE",
        )
        .bind(to_player_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        if current_faction.is_some() {
            return Err("You are already in a faction".to_string());
        }

        sqlx::query(
            r#"
            UPDATE players SET faction_id = $1, updated_at = now()
            WHERE id = $2
            "#,
        )
        .bind(faction_id)
        .bind(to_player_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'accepted', updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(invite_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'cancelled', updated_at = now()
            WHERE to_player_id = $1 AND status = 'pending' AND id <> $2
            "#,
        )
        .bind(to_player_id)
        .bind(invite_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn decline_invite(&self, invite_id: Uuid, to_player_id: Uuid) -> Result<(), String> {
        let res = sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'declined', updated_at = now()
            WHERE id = $1 AND to_player_id = $2 AND status = 'pending'
            "#,
        )
        .bind(invite_id)
        .bind(to_player_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err("Invite not found or not pending".to_string());
        }
        Ok(())
    }

    /// All pending invites for `faction_id` (outbound).
    pub async fn list_outgoing_pending_for_faction(
        &self,
        faction_id: Uuid,
    ) -> Result<Vec<FactionInviteOutgoingRow>, sqlx::Error> {
        sqlx::query_as::<_, FactionInviteOutgoingRow>(
            r#"
            SELECT
                fi.id,
                p_to.username AS to_username,
                p_from.username AS from_username,
                fi.from_player_id,
                fi.created_at
            FROM faction_invites fi
            INNER JOIN players p_to ON p_to.id = fi.to_player_id
            INNER JOIN players p_from ON p_from.id = fi.from_player_id
            WHERE fi.faction_id = $1 AND fi.status = 'pending'
            ORDER BY fi.created_at DESC
            "#,
        )
        .bind(faction_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Cancel a pending invite: inviter or faction creator.
    pub async fn cancel_invite(&self, invite_id: Uuid, actor_id: Uuid) -> Result<(), String> {
        let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<Uuid>)>(
            r#"
            SELECT fi.faction_id, fi.from_player_id, fi.status, f.creator_id
            FROM faction_invites fi
            INNER JOIN factions f ON f.id = fi.faction_id
            WHERE fi.id = $1
            "#,
        )
        .bind(invite_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Invite not found".to_string())?;

        let (_faction_id, from_player_id, status, creator_id) = row;
        if status != "pending" {
            return Err("Invite is not pending".to_string());
        }
        let is_inviter = actor_id == from_player_id;
        let is_creator = creator_id == Some(actor_id);
        if !is_inviter && !is_creator {
            return Err("Not allowed to cancel this invite".to_string());
        }

        let res = sqlx::query(
            r#"
            UPDATE faction_invites
            SET status = 'cancelled', updated_at = now()
            WHERE id = $1 AND status = 'pending'
            "#,
        )
        .bind(invite_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        if res.rows_affected() == 0 {
            return Err("Invite not found or not pending".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::faction_service::FactionService;
    use crate::db::player_service::PlayerService;
    use crate::db::test_pool;

    #[tokio::test]
    async fn test_invite_accept_flow() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool);

        let a = format!("inv_a_{}", Uuid::new_v4());
        let b = format!("inv_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();

        let fac = factions.create("Test Fac", pa.id).await.unwrap();
        players
            .set_faction_id(pa.id, Some(fac.id))
            .await
            .unwrap();

        let inv_id = invites
            .create_invite(fac.id, pa.id, pb.id)
            .await
            .expect("create");

        let list = invites.list_pending_for_player(pb.id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, inv_id);

        invites.accept_invite(inv_id, pb.id).await.expect("accept");

        let pb2 = players.get_by_id(pb.id).await.unwrap().unwrap();
        assert_eq!(pb2.faction_id, Some(fac.id));

        let list2 = invites.list_pending_for_player(pb.id).await.unwrap();
        assert!(list2.is_empty());
    }

    #[tokio::test]
    async fn test_invite_duplicate_rejected() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool);

        let a = format!("inv2_a_{}", Uuid::new_v4());
        let b = format!("inv2_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();
        let fac = factions.create("F2", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();

        invites.create_invite(fac.id, pa.id, pb.id).await.unwrap();
        let err = invites
            .create_invite(fac.id, pa.id, pb.id)
            .await
            .expect_err("dup");
        assert!(err.contains("pending") || err.contains("already"));
    }

    #[tokio::test]
    async fn test_inviter_not_in_faction_rejected() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool);

        let a = format!("inv3_a_{}", Uuid::new_v4());
        let b = format!("inv3_b_{}", Uuid::new_v4());
        let c = format!("inv3_c_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();
        let pc = players.create_player(&c, "pw").await.unwrap();
        let fac = factions.create("F3", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();

        let err = invites
            .create_invite(fac.id, pb.id, pc.id)
            .await
            .expect_err("not member");
        assert!(err.contains("not") || err.contains("faction"));
    }

    #[tokio::test]
    async fn test_cancel_pending_sent_by() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool);

        let a = format!("inv4_a_{}", Uuid::new_v4());
        let b = format!("inv4_b_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();
        let fac = factions.create("F4", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();

        invites.create_invite(fac.id, pa.id, pb.id).await.unwrap();
        invites.cancel_pending_sent_by(pa.id).await.unwrap();

        let list = invites.list_pending_for_player(pb.id).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_create_invite_only_creator_when_member_invites_off() {
        let pool = test_pool().await;
        let players = PlayerService::new(pool.clone());
        let factions = FactionService::new(pool.clone());
        let invites = FactionInviteService::new(pool.clone());

        let a = format!("inv5_a_{}", Uuid::new_v4());
        let b = format!("inv5_b_{}", Uuid::new_v4());
        let c = format!("inv5_c_{}", Uuid::new_v4());
        let pa = players.create_player(&a, "pw").await.unwrap();
        let pb = players.create_player(&b, "pw").await.unwrap();
        let pc = players.create_player(&c, "pw").await.unwrap();
        let fac = factions.create("F5", pa.id).await.unwrap();
        players.set_faction_id(pa.id, Some(fac.id)).await.unwrap();
        players.set_faction_id(pb.id, Some(fac.id)).await.unwrap();

        sqlx::query("UPDATE factions SET allow_member_invites = false WHERE id = $1")
            .bind(fac.id)
            .execute(&pool)
            .await
            .unwrap();

        let err = invites
            .create_invite(fac.id, pb.id, pc.id)
            .await
            .expect_err("non-creator should not invite");
        assert!(err.to_lowercase().contains("creator"));

        invites
            .create_invite(fac.id, pa.id, pc.id)
            .await
            .expect("creator can invite");
    }
}
