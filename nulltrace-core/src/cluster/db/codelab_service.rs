use sqlx::PgPool;
use uuid::Uuid;

pub struct CodelabService {
    pool: PgPool,
}

impl CodelabService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn mark_solved(&self, player_id: Uuid, challenge_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO codelab_progress (player_id, challenge_id)
             VALUES ($1, $2)
             ON CONFLICT (player_id, challenge_id) DO NOTHING",
            player_id,
            challenge_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_solved_ids(&self, player_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query!(
            "SELECT challenge_id FROM codelab_progress WHERE player_id = $1",
            player_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.challenge_id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::super::player_service::PlayerService;
    use super::super::test_pool;
    use super::*;
    use uuid::Uuid;

    fn unique_username() -> String {
        format!("codelab_test_{}", Uuid::new_v4())
    }

    #[tokio::test]
    async fn test_mark_solved_and_get_solved_ids() {
        let pool = test_pool().await;
        let player_service = PlayerService::new(pool.clone());
        let name = unique_username();
        let player = player_service.create_player(&name, "pw").await.unwrap();
        let codelab = CodelabService::new(pool.clone());

        // Initially no solved challenges
        let ids = codelab.get_solved_ids(player.id).await.unwrap();
        assert!(ids.is_empty());

        // Mark one solved
        codelab.mark_solved(player.id, "hello-world").await.unwrap();
        let ids = codelab.get_solved_ids(player.id).await.unwrap();
        assert_eq!(ids, vec!["hello-world".to_string()]);

        // Mark another
        codelab.mark_solved(player.id, "variables").await.unwrap();
        let ids = codelab.get_solved_ids(player.id).await.unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"hello-world".to_string()));
        assert!(ids.contains(&"variables".to_string()));

        // Idempotent: same challenge again does not duplicate (ON CONFLICT DO NOTHING)
        codelab.mark_solved(player.id, "hello-world").await.unwrap();
        let ids = codelab.get_solved_ids(player.id).await.unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn test_get_solved_ids_isolated_per_player() {
        let pool = test_pool().await;
        let player_service = PlayerService::new(pool.clone());
        let codelab = CodelabService::new(pool.clone());

        let p1 = player_service
            .create_player(&unique_username(), "pw")
            .await
            .unwrap();
        let p2 = player_service
            .create_player(&unique_username(), "pw")
            .await
            .unwrap();

        codelab.mark_solved(p1.id, "challenge-a").await.unwrap();
        codelab.mark_solved(p2.id, "challenge-b").await.unwrap();

        let ids1 = codelab.get_solved_ids(p1.id).await.unwrap();
        let ids2 = codelab.get_solved_ids(p2.id).await.unwrap();

        assert_eq!(ids1, vec!["challenge-a".to_string()]);
        assert_eq!(ids2, vec!["challenge-b".to_string()]);
    }
}
