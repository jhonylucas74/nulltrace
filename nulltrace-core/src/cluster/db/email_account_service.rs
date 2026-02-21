use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailAccountRecord {
    pub id: Uuid,
    pub player_id: Option<Uuid>,
    pub email_address: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
}

pub struct EmailAccountService {
    pool: PgPool,
}

impl EmailAccountService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create or update an email account. Uses ON CONFLICT to update the token if the address already exists.
    pub async fn create_account(
        &self,
        player_id: Option<Uuid>,
        email_address: &str,
        token: &str,
    ) -> Result<EmailAccountRecord, sqlx::Error> {
        sqlx::query_as!(
            EmailAccountRecord,
            r#"INSERT INTO email_accounts (player_id, email_address, token)
               VALUES ($1, $2, $3)
               ON CONFLICT (email_address) DO UPDATE SET token = EXCLUDED.token
               RETURNING id, player_id, email_address, token, created_at"#,
            player_id,
            email_address,
            token,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Fetch the account for the given email address.
    pub async fn get_by_email(
        &self,
        email_address: &str,
    ) -> Result<Option<EmailAccountRecord>, sqlx::Error> {
        sqlx::query_as!(
            EmailAccountRecord,
            r#"SELECT id, player_id, email_address, token, created_at
               FROM email_accounts WHERE email_address = $1"#,
            email_address,
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Returns true if the token is correct for the given address.
    pub async fn validate_token(
        &self,
        email_address: &str,
        token: &str,
    ) -> Result<bool, sqlx::Error> {
        let account = self.get_by_email(email_address).await?;
        Ok(account.map(|a| a.token == token).unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_account() {
        let pool = super::super::test_pool().await;
        let svc = EmailAccountService::new(pool);

        // player_id None: no FK to players table (NPC / unowned VM account).
        let rec = svc
            .create_account(None, "alice@test.local", "secret-token")
            .await
            .unwrap();

        assert_eq!(rec.email_address, "alice@test.local");
        assert_eq!(rec.token, "secret-token");
        assert_eq!(rec.player_id, None);
    }

    #[tokio::test]
    async fn test_get_by_email() {
        let pool = super::super::test_pool().await;
        let svc = EmailAccountService::new(pool);

        svc.create_account(None, "bob@test.local", "token-bob")
            .await
            .unwrap();

        let found = svc.get_by_email("bob@test.local").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().token, "token-bob");

        let missing = svc.get_by_email("nobody@test.local").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_validate_token() {
        let pool = super::super::test_pool().await;
        let svc = EmailAccountService::new(pool);

        svc.create_account(None, "validate@test.local", "correct-token")
            .await
            .unwrap();

        assert!(svc.validate_token("validate@test.local", "correct-token").await.unwrap());
        assert!(!svc.validate_token("validate@test.local", "wrong-token").await.unwrap());
        assert!(!svc.validate_token("unknown@test.local", "any").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_account_upsert_same_address() {
        let pool = super::super::test_pool().await;
        let svc = EmailAccountService::new(pool);

        let first = svc
            .create_account(None, "upsert@test.local", "old-token")
            .await
            .unwrap();
        let second = svc
            .create_account(Some(Uuid::new_v4()), "upsert@test.local", "new-token")
            .await
            .unwrap();

        assert_eq!(first.id, second.id, "same row updated");
        assert_eq!(second.token, "new-token");
        assert_eq!(second.email_address, "upsert@test.local");
    }
}
