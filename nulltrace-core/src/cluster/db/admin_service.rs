//! Admin user service for management API authentication.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Default admin email (seed created on startup).
pub const SEED_ADMIN_EMAIL: &str = "admin";
/// Default admin password (same as email for dev).
pub const SEED_ADMIN_PASSWORD: &str = "admin";

#[derive(Debug, Clone, FromRow)]
pub struct Admin {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AdminService {
    pool: PgPool,
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

impl AdminService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_admin(&self, email: &str, password: &str) -> Result<Admin, sqlx::Error> {
        let id = Uuid::new_v4();
        let password_hash = hash_password(password);
        let rec = sqlx::query_as::<_, Admin>(
            r#"
            INSERT INTO admins (id, email, password_hash)
            VALUES ($1, $2, $3)
            RETURNING id, email, password_hash, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<Admin>, sqlx::Error> {
        let rec = sqlx::query_as::<_, Admin>(
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM admins WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(rec)
    }

    /// Verify email and password; returns the admin if valid.
    pub async fn verify_password(&self, email: &str, password: &str) -> Result<Option<Admin>, sqlx::Error> {
        let admin = self.get_by_email(email).await?;
        let Some(admin) = admin else {
            return Ok(None);
        };
        let hash = hash_password(password);
        if admin.password_hash == hash {
            Ok(Some(admin))
        } else {
            Ok(None)
        }
    }

    /// Ensure seed admin exists. Call after migrations.
    pub async fn ensure_seed_admin(&self) -> Result<(), sqlx::Error> {
        if self.get_by_email(SEED_ADMIN_EMAIL).await?.is_some() {
            return Ok(());
        }
        self.create_admin(SEED_ADMIN_EMAIL, SEED_ADMIN_PASSWORD).await?;
        Ok(())
    }

    /// Ensure seed admin from env (ADMIN_EMAIL, ADMIN_PASSWORD) or default.
    pub async fn ensure_seed_admin_from_env(&self) -> Result<(), sqlx::Error> {
        let email = std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| SEED_ADMIN_EMAIL.to_string());
        let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| SEED_ADMIN_PASSWORD.to_string());

        if self.get_by_email(&email).await?.is_some() {
            return Ok(());
        }
        self.create_admin(&email, &password).await?;
        Ok(())
    }
}
