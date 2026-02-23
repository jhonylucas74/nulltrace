#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::wallet_common::WalletError;

const CRYPTO_CURRENCIES: &[&str] = &["BTC", "ETH", "SOL"];

#[derive(Debug, Clone, FromRow)]
pub struct CryptoWallet {
    pub id: Uuid,
    pub key_address: String,
    pub public_key: Option<String>,
    pub currency: String,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
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

pub struct CryptoWalletService {
    pool: PgPool,
}

impl CryptoWalletService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register a crypto address (no owner). Idempotent by key_address.
    pub async fn register(
        &self,
        key_address: &str,
        public_key: Option<&str>,
        currency: &str,
    ) -> Result<CryptoWallet, WalletError> {
        if !CRYPTO_CURRENCIES.contains(&currency) {
            return Err(WalletError::InvalidCurrency);
        }
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO crypto_wallets (id, key_address, public_key, currency, balance, created_at)
            VALUES ($1, $2, $3, $4, 0, now())
            ON CONFLICT (key_address) DO UPDATE SET public_key = COALESCE(EXCLUDED.public_key, crypto_wallets.public_key)
            RETURNING id, key_address, public_key, currency, balance, created_at
            "#,
        )
        .bind(id)
        .bind(key_address)
        .bind(public_key)
        .bind(currency)
        .fetch_one(&self.pool)
        .await
        .map_err(WalletError::Db)?;

        self.get_by_address(key_address).await
    }

    pub async fn get_by_address(&self, key_address: &str) -> Result<CryptoWallet, WalletError> {
        let row = sqlx::query_as::<_, CryptoWallet>(
            "SELECT id, key_address, public_key, currency, balance, created_at FROM crypto_wallets WHERE key_address = $1",
        )
        .bind(key_address)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or(WalletError::InvalidCurrency)
    }

    pub async fn get_balance(&self, key_address: &str) -> Result<i64, WalletError> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT balance FROM crypto_wallets WHERE key_address = $1",
        )
        .bind(key_address)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|r| r.0).ok_or(WalletError::InvalidCurrency)
    }

    /// History by address. No auth required (crypto is not traceable).
    pub async fn history_by_address(
        &self,
        address: &str,
        filter: &str,
    ) -> Result<Vec<KeyBasedTransaction>, WalletError> {
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
                WHERE currency != 'USD' AND (from_key = $1 OR to_key = $1) AND created_at >= $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(address)
            .bind(after)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, KeyBasedTransaction>(
                r#"
                SELECT id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at
                FROM wallet_transactions
                WHERE currency != 'USD' AND (from_key = $1 OR to_key = $1)
                ORDER BY created_at DESC
                "#,
            )
            .bind(address)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    /// Credit address (e.g. incoming transfer). Inserts transaction.
    pub async fn credit(
        &self,
        currency: &str,
        key_address: &str,
        amount: i64,
        description: &str,
        from_key: &str,
    ) -> Result<(), WalletError> {
        let updated = sqlx::query(
            "UPDATE crypto_wallets SET balance = balance + $1 WHERE key_address = $2",
        )
        .bind(amount)
        .bind(key_address)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            // Address might not exist; create wallet if it's a known crypto currency
            if CRYPTO_CURRENCIES.contains(&currency) {
                self.register(key_address, None, currency).await?;
                sqlx::query(
                    "UPDATE crypto_wallets SET balance = balance + $1 WHERE key_address = $2",
                )
                .bind(amount)
                .bind(key_address)
                .execute(&self.pool)
                .await?;
            } else {
                return Err(WalletError::InvalidCurrency);
            }
        }
        self.insert_tx(currency, amount, 0, description, from_key, key_address, None)
            .await?;
        Ok(())
    }

    /// Debit from address. Caller must prove ownership via private_key_content (verified that it matches from_address).
    /// For now we only check that from_address exists and has sufficient balance; proper crypto verification can be added.
    pub async fn transfer(
        &self,
        currency: &str,
        from_address: &str,
        to_address: &str,
        amount: i64,
        _private_key_content: &str,
    ) -> Result<(), WalletError> {
        if from_address == to_address {
            return Err(WalletError::InvalidCurrency);
        }
        // TODO: verify _private_key_content derives to from_address (e.g. via secp256k1)
        let mut tx = self.pool.begin().await?;
        let deducted = sqlx::query(
            r#"
            UPDATE crypto_wallets SET balance = balance - $1
            WHERE key_address = $2 AND currency = $3 AND balance >= $1
            "#,
        )
        .bind(amount)
        .bind(from_address)
        .bind(currency)
        .execute(&mut *tx)
        .await?;
        if deducted.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }
        // Credit to_address (may be same currency crypto or external)
        let _ = sqlx::query(
            "UPDATE crypto_wallets SET balance = balance + $1 WHERE key_address = $2",
        )
        .bind(amount)
        .bind(to_address)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at)
            VALUES ($1, $2, $3, 0, 'Transfer', $4, $5, $6, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(currency)
        .bind(amount)
        .bind(from_address)
        .bind(to_address)
        .bind(Some(to_address))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn insert_tx(
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::wallet_common::{generate_btc_address, generate_eth_address, generate_sol_address};

    #[tokio::test]
    async fn test_register_and_balance_and_transfer() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        svc.register(&addr, None, "BTC").await.unwrap();
        let balance = svc.get_balance(&addr).await.unwrap();
        assert_eq!(balance, 0);

        svc.credit("BTC", &addr, 1000, "test", "system").await.unwrap();
        let balance = svc.get_balance(&addr).await.unwrap();
        assert_eq!(balance, 1000);

        let addr2 = generate_btc_address();
        svc.register(&addr2, None, "BTC").await.unwrap();
        svc.transfer("BTC", &addr, &addr2, 400, "dummy_priv").await.unwrap();
        assert_eq!(svc.get_balance(&addr).await.unwrap(), 600);
        assert_eq!(svc.get_balance(&addr2).await.unwrap(), 400);

        let history = svc.history_by_address(&addr, "").await.unwrap();
        assert!(history.len() >= 2);
    }

    #[tokio::test]
    async fn test_register_invalid_currency_fails() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        let res = svc.register(&addr, None, "USD").await;
        assert!(res.is_err());
        let res = svc.register(&addr, None, "INVALID").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_balance_nonexistent_address_fails() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        let res = svc.get_balance(&addr).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_history_by_address_empty() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        svc.register(&addr, None, "BTC").await.unwrap();
        let history = svc.history_by_address(&addr, "").await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_transfer_same_address_fails() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        svc.register(&addr, None, "BTC").await.unwrap();
        svc.credit("BTC", &addr, 1000, "seed", "system").await.unwrap();
        let res = svc.transfer("BTC", &addr, &addr, 100, "dummy").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_transfer_insufficient_balance_fails() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr_a = generate_btc_address();
        let addr_b = generate_btc_address();

        svc.register(&addr_a, None, "BTC").await.unwrap();
        svc.register(&addr_b, None, "BTC").await.unwrap();
        svc.credit("BTC", &addr_a, 50, "seed", "system").await.unwrap();
        let res = svc.transfer("BTC", &addr_a, &addr_b, 100, "dummy").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_register_idempotent_same_address() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_btc_address();

        svc.register(&addr, None, "BTC").await.unwrap();
        svc.register(&addr, Some("pubkey"), "BTC").await.unwrap();
        let balance = svc.get_balance(&addr).await.unwrap();
        assert_eq!(balance, 0);
    }

    #[tokio::test]
    async fn test_credit_creates_wallet_for_unknown_address() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_eth_address();

        svc.credit("ETH", &addr, 500, "airdrop", "system").await.unwrap();
        assert_eq!(svc.get_balance(&addr).await.unwrap(), 500);
        let history = svc.history_by_address(&addr, "").await.unwrap();
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_history_by_address_with_filter() {
        let pool = super::super::test_pool().await;
        let svc = CryptoWalletService::new(pool);
        let addr = generate_sol_address();

        svc.register(&addr, None, "SOL").await.unwrap();
        svc.credit("SOL", &addr, 100, "a", "system").await.unwrap();
        let all = svc.history_by_address(&addr, "").await.unwrap();
        let today = svc.history_by_address(&addr, "today").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(today.len(), 1);
    }
}
