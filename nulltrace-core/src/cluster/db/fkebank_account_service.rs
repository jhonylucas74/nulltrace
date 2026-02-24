#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool, Transaction};
use uuid::Uuid;

use super::wallet_common::{generate_fkebank_key, WalletError};

#[derive(Debug, Clone, FromRow)]
pub struct FkebankAccount {
    pub id: Uuid,
    pub owner_type: String,
    pub owner_id: Uuid,
    pub key: String,
    pub full_name: Option<String>,
    pub document_id: Option<String>,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct KeyBasedTransaction {
    pub id: Uuid,
    pub currency: String,
    pub amount: i64,
    pub fee: i64,
    pub description: Option<String>,
    pub from_key: String,
    pub to_key: String,
    pub counterpart_key: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct FkebankAccountService {
    pool: PgPool,
}

impl FkebankAccountService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a USD account for a player or VM. Idempotent (one account per owner).
    pub async fn create_account_for_owner(
        &self,
        owner_type: &str,
        owner_id: Uuid,
        full_name: Option<&str>,
        document_id: Option<&str>,
    ) -> Result<FkebankAccount, WalletError> {
        let key = generate_fkebank_key();
        let id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query_as::<_, FkebankAccount>(
            r#"
            INSERT INTO fkebank_accounts (id, owner_type, owner_id, key, full_name, document_id, balance, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $7)
            ON CONFLICT (owner_type, owner_id) DO UPDATE SET updated_at = $7
            RETURNING id, owner_type, owner_id, key, full_name, document_id, balance, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(owner_type)
        .bind(owner_id)
        .bind(&key)
        .bind(full_name)
        .bind(document_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(WalletError::Db)
    }

    pub async fn get_by_owner(
        &self,
        owner_type: &str,
        owner_id: Uuid,
    ) -> Result<FkebankAccount, WalletError> {
        let row = sqlx::query_as::<_, FkebankAccount>(
            "SELECT id, owner_type, owner_id, key, full_name, document_id, balance, created_at, updated_at FROM fkebank_accounts WHERE owner_type = $1 AND owner_id = $2",
        )
        .bind(owner_type)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or(WalletError::InvalidCurrency)
    }

    pub async fn get_by_key(&self, key: &str) -> Result<Option<FkebankAccount>, WalletError> {
        let row = sqlx::query_as::<_, FkebankAccount>(
            "SELECT id, owner_type, owner_id, key, full_name, document_id, balance, created_at, updated_at FROM fkebank_accounts WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Create token for account. One token per account; overwrites if exists.
    pub async fn create_token(&self, account_id: Uuid) -> Result<String, WalletError> {
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        sqlx::query(
            r#"
            INSERT INTO fkebank_tokens (id, account_id, token, created_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (account_id) DO UPDATE SET token = EXCLUDED.token
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind(&token)
        .execute(&self.pool)
        .await?;
        Ok(token)
    }

    /// Validate token for the given account key. Returns true if valid.
    pub async fn validate_token(&self, account_key: &str, token: &str) -> Result<bool, WalletError> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT t.token FROM fkebank_tokens t JOIN fkebank_accounts a ON a.id = t.account_id WHERE a.key = $1",
        )
        .bind(account_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0 == token).unwrap_or(false))
    }

    /// Get account key (PIX key) for a token. Returns None if token is invalid.
    pub async fn get_key_by_token(&self, token: &str) -> Result<Option<String>, WalletError> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT a.key FROM fkebank_accounts a JOIN fkebank_tokens t ON t.account_id = a.id WHERE t.token = $1",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// Get balance (cents) by account key.
    pub async fn get_balance_by_key(&self, key: &str) -> Result<i64, WalletError> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT balance FROM fkebank_accounts WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.0).ok_or(WalletError::InvalidCurrency)
    }

    /// Credit account by key. Inserts a key-based transaction (from_key='system', to_key=key).
    pub async fn credit(&self, key: &str, amount: i64, description: &str) -> Result<(), WalletError> {
        let updated = sqlx::query(
            "UPDATE fkebank_accounts SET balance = balance + $1, updated_at = now() WHERE key = $2",
        )
        .bind(amount)
        .bind(key)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(WalletError::InvalidCurrency);
        }
        self.insert_transaction("USD", amount, 0, description, "system", key, None).await?;
        Ok(())
    }

    /// Debit account by key. Fails if insufficient balance.
    pub async fn debit(&self, key: &str, amount: i64, description: &str) -> Result<(), WalletError> {
        self.debit_to_key(key, amount, description, "system").await
    }

    /// Debit and record transaction with a specific to_key (e.g. external address for transfer_out).
    pub async fn debit_to_key(
        &self,
        key: &str,
        amount: i64,
        description: &str,
        to_key: &str,
    ) -> Result<(), WalletError> {
        let updated = sqlx::query(
            r#"
            UPDATE fkebank_accounts SET balance = balance - $1, updated_at = now()
            WHERE key = $2 AND balance >= $1
            "#,
        )
        .bind(amount)
        .bind(key)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(WalletError::InsufficientBalance);
        }
        self.insert_transaction("USD", amount, 0, description, key, to_key, Some(to_key))
            .await?;
        Ok(())
    }

    /// Transfer USD from from_key to to_key. Validates token for from_key if token is provided.
    pub async fn transfer(
        &self,
        from_key: &str,
        to_key: &str,
        amount: i64,
        token: Option<&str>,
    ) -> Result<(), WalletError> {
        if from_key == to_key {
            return Err(WalletError::InvalidCurrency);
        }
        if let Some(t) = token {
            let valid = self.validate_token(from_key, t).await?;
            if !valid {
                return Err(WalletError::InvalidCurrency);
            }
        }
        let mut tx = self.pool.begin().await?;
        let deducted = sqlx::query(
            r#"
            UPDATE fkebank_accounts SET balance = balance - $1, updated_at = now()
            WHERE key = $2 AND balance >= $1
            "#,
        )
        .bind(amount)
        .bind(from_key)
        .execute(&mut *tx)
        .await?;
        if deducted.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }
        // Credit to_key; if recipient does not exist, rollback so money is never lost.
        let credited = sqlx::query(
            "UPDATE fkebank_accounts SET balance = balance + $1, updated_at = now() WHERE key = $2",
        )
        .bind(amount)
        .bind(to_key)
        .execute(&mut *tx)
        .await?;
        if credited.rows_affected() != 1 {
            tx.rollback().await?;
            return Err(WalletError::RecipientNotFound);
        }
        self.insert_transaction_tx(&mut *tx, "USD", amount, 0, "Transfer", from_key, to_key, Some(to_key.to_string())).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn insert_transaction(
        &self,
        currency: &str,
        amount: i64,
        fee: i64,
        description: &str,
        from_key: &str,
        to_key: &str,
        counterpart_key: Option<&str>,
    ) -> Result<(), WalletError> {
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(currency)
        .bind(amount)
        .bind(fee)
        .bind(description)
        .bind(from_key)
        .bind(to_key)
        .bind(counterpart_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_transaction_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        currency: &str,
        amount: i64,
        fee: i64,
        description: &str,
        from_key: &str,
        to_key: &str,
        counterpart_key: Option<String>,
    ) -> Result<(), WalletError> {
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(currency)
        .bind(amount)
        .bind(fee)
        .bind(description)
        .bind(from_key)
        .bind(to_key)
        .bind(counterpart_key)
        .execute(tx)
        .await?;
        Ok(())
    }

    /// Debit key within an existing transaction (for atomic convert). Caller commits or rolls back.
    pub async fn debit_on_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        key: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        let updated = sqlx::query(
            r#"
            UPDATE fkebank_accounts SET balance = balance - $1, updated_at = now()
            WHERE key = $2 AND balance >= $1
            "#,
        )
        .bind(amount)
        .bind(key)
        .execute(&mut **tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(WalletError::InsufficientBalance);
        }
        self.insert_transaction_tx(&mut **tx, "USD", amount, 0, description, key, "system", Some("system".to_string()))
            .await?;
        Ok(())
    }

    /// Credit key within an existing transaction (for atomic convert). Caller commits or rolls back.
    pub async fn credit_on_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
        key: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        let updated = sqlx::query(
            "UPDATE fkebank_accounts SET balance = balance + $1, updated_at = now() WHERE key = $2",
        )
        .bind(amount)
        .bind(key)
        .execute(&mut **tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(WalletError::InvalidCurrency);
        }
        self.insert_transaction_tx(&mut **tx, "USD", amount, 0, description, "system", key, None).await?;
        Ok(())
    }

    /// History for a key. USD requires token validation.
    pub async fn history_by_key(
        &self,
        key: &str,
        token: Option<&str>,
        filter: &str,
    ) -> Result<Vec<KeyBasedTransaction>, WalletError> {
        if let Some(t) = token {
            let valid = self.validate_token(key, t).await?;
            if !valid {
                return Err(WalletError::InvalidCurrency);
            }
        }
        let cutoff: Option<DateTime<Utc>> = match filter {
            "today" => {
                let now = Utc::now();
                Some(now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc())
            }
            "7d" => Some(Utc::now() - Duration::days(7)),
            "30d" => Some(Utc::now() - Duration::days(30)),
            _ => None,
        };
        let rows = if let Some(after) = cutoff {
            sqlx::query_as::<_, KeyBasedTransaction>(
                r#"
                SELECT id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at
                FROM wallet_transactions
                WHERE currency = 'USD' AND (from_key = $1 OR to_key = $1) AND created_at >= $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(key)
            .bind(after)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, KeyBasedTransaction>(
                r#"
                SELECT id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at
                FROM wallet_transactions
                WHERE currency = 'USD' AND (from_key = $1 OR to_key = $1)
                ORDER BY created_at DESC
                "#,
            )
            .bind(key)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_key_by_token_and_balance() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        let token = svc.create_token(account.id).await.unwrap();

        let key = svc.get_key_by_token(&token).await.unwrap();
        assert_eq!(key.as_deref(), Some(account.key.as_str()));

        svc.credit(&account.key, 5000, "test").await.unwrap();
        let balance = svc.get_balance_by_key(&account.key).await.unwrap();
        assert_eq!(balance, 5000);

        assert!(svc.get_key_by_token("invalid-token").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_validate_token_correct_and_incorrect() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        let token = svc.create_token(account.id).await.unwrap();

        let valid = svc.validate_token(&account.key, &token).await.unwrap();
        assert!(valid);
        assert!(!svc.validate_token(&account.key, "wrong").await.unwrap());
        assert!(!svc.validate_token("fkebank-nonexistent", &token).await.unwrap());
    }

    #[tokio::test]
    async fn test_transfer_with_token_internal() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let a_id = Uuid::new_v4();
        let b_id = Uuid::new_v4();

        let acc_a = svc.create_account_for_owner("player", a_id, None, None).await.unwrap();
        let acc_b = svc.create_account_for_owner("player", b_id, None, None).await.unwrap();
        svc.credit(&acc_a.key, 10000, "seed").await.unwrap();
        let token_a = svc.create_token(acc_a.id).await.unwrap();

        svc.transfer(&acc_a.key, &acc_b.key, 3000, Some(&token_a)).await.unwrap();

        assert_eq!(svc.get_balance_by_key(&acc_a.key).await.unwrap(), 7000);
        assert_eq!(svc.get_balance_by_key(&acc_b.key).await.unwrap(), 3000);
    }

    #[tokio::test]
    async fn test_history_by_key_with_filter() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        svc.credit(&account.key, 1000, "first").await.unwrap();
        svc.credit(&account.key, 2000, "second").await.unwrap();

        let all = svc.history_by_key(&account.key, None, "").await.unwrap();
        assert_eq!(all.len(), 2);
        let today = svc.history_by_key(&account.key, None, "today").await.unwrap();
        assert_eq!(today.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_owner_nonexistent_fails() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let random_id = Uuid::new_v4();

        let res = svc.get_by_owner("player", random_id).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_create_account_with_full_name_and_document() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc
            .create_account_for_owner("vm", vm_id, Some("Money Null"), Some("doc-123"))
            .await
            .unwrap();
        assert_eq!(account.full_name.as_deref(), Some("Money Null"));
        assert_eq!(account.document_id.as_deref(), Some("doc-123"));
    }

    #[tokio::test]
    async fn test_debit_to_key_records_to_key_in_transaction() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        svc.credit(&account.key, 5000, "seed").await.unwrap();
        svc.debit_to_key(&account.key, 1000, "External", "external-addr").await.unwrap();

        let history = svc.history_by_key(&account.key, None, "").await.unwrap();
        let debit_tx = history.iter().find(|t| t.from_key == account.key && t.to_key == "external-addr").unwrap();
        assert_eq!(debit_tx.amount, 1000);
    }

    #[tokio::test]
    async fn test_transfer_same_key_fails() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        svc.credit(&account.key, 1000, "seed").await.unwrap();
        let res = svc.transfer(&account.key, &account.key, 100, None).await;
        assert!(res.is_err());
    }

    /// Transfer to a key that does not exist: must return error and leave sender balance unchanged.
    #[tokio::test]
    async fn test_transfer_to_nonexistent_recipient_fails() {
        let pool = super::super::test_pool().await;
        let svc = FkebankAccountService::new(pool);
        let vm_id = Uuid::new_v4();

        let account = svc.create_account_for_owner("vm", vm_id, None, None).await.unwrap();
        svc.credit(&account.key, 5000, "seed").await.unwrap();

        let res = svc.transfer(&account.key, "fkebank-nonexistent-key", 1000, None).await;
        assert!(matches!(res, Err(super::super::wallet_common::WalletError::RecipientNotFound)));

        let balance = svc.get_balance_by_key(&account.key).await.unwrap();
        assert_eq!(balance, 5000);
    }
}
