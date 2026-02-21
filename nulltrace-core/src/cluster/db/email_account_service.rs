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
