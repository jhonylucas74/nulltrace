use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailRecord {
    pub id: Uuid,
    pub from_address: String,
    pub to_address: String,
    pub subject: String,
    pub body: String,
    pub folder: String,
    pub read: bool,
    pub sent_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct EmailService {
    pool: PgPool,
}

impl EmailService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new email into the given folder (default 'inbox').
    pub async fn insert_email(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        folder: &str,
    ) -> Result<EmailRecord, sqlx::Error> {
        sqlx::query_as!(
            EmailRecord,
            r#"INSERT INTO emails (from_address, to_address, subject, body, folder)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, from_address, to_address, subject, body, folder, read, sent_at, created_at"#,
            from,
            to,
            subject,
            body,
            folder,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// List emails for the given address in the given folder, newest first.
    pub async fn list_emails(
        &self,
        email_address: &str,
        folder: &str,
    ) -> Result<Vec<EmailRecord>, sqlx::Error> {
        sqlx::query_as!(
            EmailRecord,
            r#"SELECT id, from_address, to_address, subject, body, folder, read, sent_at, created_at
               FROM emails
               WHERE to_address = $1 AND folder = $2
               ORDER BY sent_at DESC"#,
            email_address,
            folder,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Update the read status of an email.
    pub async fn mark_read(&self, email_id: Uuid, read: bool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE emails SET read = $1 WHERE id = $2",
            read,
            email_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Move an email to a different folder.
    pub async fn move_to_folder(&self, email_id: Uuid, folder: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE emails SET folder = $1 WHERE id = $2",
            folder,
            email_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Permanently delete an email.
    pub async fn delete_email(&self, email_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM emails WHERE id = $1", email_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Count unread emails in the inbox for the given address.
    pub async fn unread_count(&self, email_address: &str) -> Result<i64, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM emails WHERE to_address = $1 AND read = false AND folder = 'inbox'",
            email_address,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.count.unwrap_or(0))
    }
}
