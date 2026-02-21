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
    pub cc_address: Option<String>,
}

pub struct EmailService {
    pool: PgPool,
}

impl EmailService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new email into the given folder (default 'inbox').
    /// cc_address is optional; set for the main recipient row and sent copy so the UI can display CC.
    pub async fn insert_email(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        folder: &str,
        cc_address: Option<&str>,
    ) -> Result<EmailRecord, sqlx::Error> {
        let cc = cc_address.and_then(|s| if s.is_empty() { None } else { Some(s) });
        sqlx::query_as!(
            EmailRecord,
            r#"INSERT INTO emails (from_address, to_address, subject, body, folder, cc_address)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, from_address, to_address, subject, body, folder, read, sent_at, created_at, cc_address"#,
            from,
            to,
            subject,
            body,
            folder,
            cc,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Fixed page size for email listing (security: no unbounded responses).
    const PAGE_SIZE: i64 = 50;

    /// List one page of emails for the given address and folder (0-based page).
    /// Inbox/spam/trash: to_address = owner. Sent: from_address = owner.
    /// Returns (records for this page, has_more).
    pub async fn list_emails_page(
        &self,
        email_address: &str,
        folder: &str,
        page: i32,
    ) -> Result<(Vec<EmailRecord>, bool), sqlx::Error> {
        let page = page.max(0) as i64;
        let limit = Self::PAGE_SIZE + 1;
        let offset = page * Self::PAGE_SIZE;

        const SENT_QUERY: &str = r#"SELECT id, from_address, to_address, subject, body, folder, read, sent_at, created_at, cc_address
            FROM emails WHERE from_address = $1 AND folder = $2 ORDER BY sent_at DESC LIMIT $3 OFFSET $4"#;
        const INBOX_QUERY: &str = r#"SELECT id, from_address, to_address, subject, body, folder, read, sent_at, created_at, cc_address
            FROM emails WHERE to_address = $1 AND folder = $2 ORDER BY sent_at DESC LIMIT $3 OFFSET $4"#;

        let rows: Vec<EmailRecord> = if folder == "sent" {
            sqlx::query_as::<_, EmailRecord>(SENT_QUERY)
                .bind(email_address)
                .bind(folder)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, EmailRecord>(INBOX_QUERY)
                .bind(email_address)
                .bind(folder)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };

        let has_more = rows.len() as i64 > Self::PAGE_SIZE;
        let page_records = if has_more {
            rows.into_iter().take(Self::PAGE_SIZE as usize).collect()
        } else {
            rows
        };
        Ok((page_records, has_more))
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// Unique address per test to avoid shared DB state when tests run in sequence.
    fn unique(prefix: &str) -> String {
        format!("{}-{}@test.local", prefix, Uuid::new_v4())
    }

    #[tokio::test]
    async fn test_insert_email_and_list_inbox() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("inbox");

        let rec = svc
            .insert_email("sender@test.local", &to_addr, "Hello", "Body text", "inbox", None)
            .await
            .unwrap();

        assert_eq!(rec.from_address, "sender@test.local");
        assert_eq!(rec.to_address, to_addr);
        assert_eq!(rec.subject, "Hello");
        assert_eq!(rec.folder, "inbox");
        assert!(!rec.read);

        let (page, has_more) = svc.list_emails_page(&to_addr, "inbox", 0).await.unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, rec.id);
        assert!(!has_more);
    }

    #[tokio::test]
    async fn test_list_emails_sent() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let from_addr = unique("sent");

        svc.insert_email(
            &from_addr,
            "other@test.local",
            "Sent subject",
            "Sent body",
            "sent",
            None,
        )
        .await
        .unwrap();

        let (page, has_more) = svc.list_emails_page(&from_addr, "sent", 0).await.unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].subject, "Sent subject");
        assert!(!has_more);

        let (inbox_page, _) = svc.list_emails_page(&from_addr, "inbox", 0).await.unwrap();
        assert!(inbox_page.is_empty());
    }

    #[tokio::test]
    async fn test_list_emails_page_pagination() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("paged");

        for i in 0..51 {
            svc.insert_email(
                "from@test.local",
                &to_addr,
                &format!("Subject {}", i),
                "body",
                "inbox",
                None,
            )
            .await
            .unwrap();
        }

        let (page0, has_more0) = svc.list_emails_page(&to_addr, "inbox", 0).await.unwrap();
        assert_eq!(page0.len(), 50);
        assert!(has_more0);

        let (page1, has_more1) = svc.list_emails_page(&to_addr, "inbox", 1).await.unwrap();
        assert_eq!(page1.len(), 1);
        assert!(!has_more1);
    }

    #[tokio::test]
    async fn test_mark_read() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("markread");

        let rec = svc
            .insert_email("a@t.local", &to_addr, "Subj", "Body", "inbox", None)
            .await
            .unwrap();
        assert!(!rec.read);

        svc.mark_read(rec.id, true).await.unwrap();

        let (page, _) = svc.list_emails_page(&to_addr, "inbox", 0).await.unwrap();
        assert_eq!(page[0].id, rec.id);
        assert!(page[0].read);
    }

    #[tokio::test]
    async fn test_move_to_folder() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("move");

        let rec = svc
            .insert_email("a@t.local", &to_addr, "Subj", "Body", "inbox", None)
            .await
            .unwrap();

        svc.move_to_folder(rec.id, "trash").await.unwrap();

        let (inbox, _) = svc.list_emails_page(&to_addr, "inbox", 0).await.unwrap();
        assert!(inbox.is_empty());

        let (trash, _) = svc.list_emails_page(&to_addr, "trash", 0).await.unwrap();
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].folder, "trash");
    }

    #[tokio::test]
    async fn test_delete_email() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("del");

        let rec = svc
            .insert_email("a@t.local", &to_addr, "Subj", "Body", "inbox", None)
            .await
            .unwrap();

        svc.delete_email(rec.id).await.unwrap();

        let (page, _) = svc.list_emails_page(&to_addr, "inbox", 0).await.unwrap();
        assert!(page.is_empty());
    }

    #[tokio::test]
    async fn test_unread_count() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("unread");

        let r1 = svc
            .insert_email("a@t.local", &to_addr, "S1", "B1", "inbox", None)
            .await
            .unwrap();
        let _r2 = svc
            .insert_email("a@t.local", &to_addr, "S2", "B2", "inbox", None)
            .await
            .unwrap();

        assert_eq!(svc.unread_count(&to_addr).await.unwrap(), 2);

        svc.mark_read(r1.id, true).await.unwrap();
        assert_eq!(svc.unread_count(&to_addr).await.unwrap(), 1);

        svc.move_to_folder(r1.id, "trash").await.unwrap();
        assert_eq!(svc.unread_count(&to_addr).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_insert_email_with_cc() {
        let pool = super::super::test_pool().await;
        let svc = EmailService::new(pool);
        let to_addr = unique("cc");

        let rec = svc
            .insert_email(
                "from@test.local",
                &to_addr,
                "CC test",
                "Body",
                "inbox",
                Some("cc@test.local"),
            )
            .await
            .unwrap();

        assert_eq!(rec.cc_address.as_deref(), Some("cc@test.local"));
    }
}
