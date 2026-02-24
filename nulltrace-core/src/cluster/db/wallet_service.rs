#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use super::crypto_wallet_service::CryptoWalletService;
use super::fkebank_account_service::FkebankAccountService;

// Re-export for grpc, wallet_card_service, and tests
pub use super::wallet_common::{
    convert_amount, generate_btc_address, generate_eth_address, generate_fkebank_key,
    generate_sol_address, WalletError,
};

#[derive(Debug, Clone)]
pub struct WalletBalance {
    pub currency: String,
    pub balance: i64,
}

#[derive(Debug, Clone)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub player_id: Uuid,
    pub tx_type: String,
    pub currency: String,
    pub amount: i64,
    pub fee: i64,
    pub description: Option<String>,
    pub counterpart_address: Option<String>,
    pub counterpart_player_id: Option<Uuid>,
    pub related_transaction_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WalletKey {
    pub currency: String,
    pub key_address: String,
}

const CURRENCIES: &[&str] = &["USD", "BTC", "ETH", "SOL"];

fn player_crypto_vault_key(player_id: Uuid, currency: &str) -> String {
    format!("player-vault-{}-{}", player_id, currency)
}

pub struct WalletService {
    pool: PgPool,
    fkebank: Arc<FkebankAccountService>,
    crypto: Arc<CryptoWalletService>,
}

impl WalletService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            fkebank: Arc::new(FkebankAccountService::new(pool.clone())),
            crypto: Arc::new(CryptoWalletService::new(pool)),
        }
    }

    pub fn fkebank_service(&self) -> Arc<FkebankAccountService> {
        self.fkebank.clone()
    }

    pub fn crypto_service(&self) -> Arc<CryptoWalletService> {
        self.crypto.clone()
    }

    /// Creates Fkebank USD account for player (and token). Idempotent.
    pub async fn create_wallet_for_player(&self, player_id: Uuid) -> Result<(), WalletError> {
        let account = self
            .fkebank
            .create_account_for_owner("player", player_id, None, None)
            .await?;
        let _ = self.fkebank.create_token(account.id).await?;
        Ok(())
    }

    /// Creates Fkebank USD account for a VM (and token). Idempotent. Returns (account_key, token) for writing to VM fs.
    pub async fn create_wallet_for_vm(&self, vm_id: Uuid) -> Result<(String, String), WalletError> {
        let account = self
            .fkebank
            .create_account_for_owner("vm", vm_id, None, None)
            .await?;
        let token = self.fkebank.create_token(account.id).await?;
        Ok((account.key, token))
    }

    /// Returns balances: USD from Fkebank account, BTC/ETH/SOL from player crypto vaults (created on first use).
    pub async fn get_balances(&self, player_id: Uuid) -> Result<Vec<WalletBalance>, WalletError> {
        let account = self.fkebank.get_by_owner("player", player_id).await.ok();
        let usd_balance = account.as_ref().map(|a| a.balance).unwrap_or(0);
        let mut out = Vec::with_capacity(CURRENCIES.len());
        for c in CURRENCIES {
            let balance = if *c == "USD" {
                usd_balance
            } else {
                let key = player_crypto_vault_key(player_id, c);
                let _ = self.crypto.register(&key, None, c).await?;
                self.crypto.get_balance(&key).await.unwrap_or(0)
            };
            out.push(WalletBalance {
                currency: (*c).to_string(),
                balance,
            });
        }
        Ok(out)
    }

    /// Returns all wallet transactions (USD + BTC/ETH/SOL) for the player, newest first.
    pub async fn get_transactions(
        &self,
        player_id: Uuid,
        filter: &str,
    ) -> Result<Vec<WalletTransaction>, WalletError> {
        let account = self.fkebank.get_by_owner("player", player_id).await?;
        let usd_key = account.key.clone();

        let mut out: Vec<WalletTransaction> = Vec::new();

        // USD: Fkebank history
        let usd_rows = self.fkebank.history_by_key(&usd_key, None, filter).await?;
        for r in usd_rows {
            let tx_type = if r.to_key == usd_key && r.from_key == "system" {
                "credit"
            } else if r.from_key == usd_key && r.to_key == "system" {
                "debit"
            } else if r.to_key == usd_key {
                "transfer_in"
            } else {
                "transfer_out"
            };
            out.push(WalletTransaction {
                id: r.id,
                player_id,
                tx_type: tx_type.to_string(),
                currency: r.currency,
                amount: r.amount,
                fee: r.fee,
                description: r.description,
                counterpart_address: Some(if r.to_key == usd_key {
                    r.from_key
                } else {
                    r.to_key
                }),
                counterpart_player_id: None,
                related_transaction_id: None,
                created_at: r.created_at,
            });
        }

        // Crypto: history for each vault address
        for currency in &["BTC", "ETH", "SOL"] {
            let vault_key = player_crypto_vault_key(player_id, currency);
            let _ = self.crypto.register(&vault_key, None, currency).await?;
            let rows = self.crypto.history_by_address(&vault_key, filter).await?;
            for r in rows {
                let tx_type = if r.to_key == vault_key && r.from_key == "system" {
                    "credit"
                } else if r.from_key == vault_key && r.to_key == "system" {
                    "debit"
                } else if r.to_key == vault_key {
                    "transfer_in"
                } else {
                    "transfer_out"
                };
                out.push(WalletTransaction {
                    id: r.id,
                    player_id,
                    tx_type: tx_type.to_string(),
                    currency: r.currency.clone(),
                    amount: r.amount,
                    fee: r.fee,
                    description: r.description,
                    counterpart_address: Some(if r.to_key == vault_key {
                        r.from_key
                    } else {
                        r.to_key
                    }),
                    counterpart_player_id: None,
                    related_transaction_id: None,
                    created_at: r.created_at,
                });
            }
        }

        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(out)
    }

    pub async fn credit(
        &self,
        player_id: Uuid,
        currency: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        if currency != "USD" {
            return Err(WalletError::InvalidCurrency);
        }
        let account = self.fkebank.get_by_owner("player", player_id).await?;
        self.fkebank.credit(&account.key, amount, description).await
    }

    pub async fn debit(
        &self,
        player_id: Uuid,
        currency: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        if currency != "USD" {
            return Err(WalletError::InvalidCurrency);
        }
        let account = self.fkebank.get_by_owner("player", player_id).await?;
        self.fkebank.debit(&account.key, amount, description).await
    }

    /// Transfer to address. USD: Fkebank internal or external debit. Crypto: debit player vault, credit target address.
    pub async fn transfer_to_address(
        &self,
        player_id: Uuid,
        target_address: &str,
        currency: &str,
        amount: i64,
    ) -> Result<(), WalletError> {
        let target = target_address.trim();
        if target.is_empty() {
            return Err(WalletError::InvalidCurrency);
        }
        if amount <= 0 {
            return Err(WalletError::InsufficientBalance);
        }
        if currency == "USD" {
            let account = self.fkebank.get_by_owner("player", player_id).await?;
            let from_key = account.key.as_str();
            if from_key == target {
                return Err(WalletError::InvalidCurrency);
            }
            if let Some(to_acc) = self.fkebank.get_by_key(target).await? {
                self.fkebank.transfer(from_key, &to_acc.key, amount, None).await
            } else {
                self.fkebank
                    .debit_to_key(from_key, amount, "External transfer", target)
                    .await
            }
        } else if CURRENCIES.contains(&currency) && currency != "USD" {
            // Crypto: debit from player vault, credit to target in one atomic transaction
            let from_key = player_crypto_vault_key(player_id, currency);
            if from_key == target {
                return Err(WalletError::InvalidCurrency);
            }
            let _ = self.crypto.register(&from_key, None, currency).await?;
            let balance = self.crypto.get_balance(&from_key).await.unwrap_or(0);
            if balance < amount {
                return Err(WalletError::InsufficientBalance);
            }
            self.crypto
                .transfer_atomic(currency, &from_key, target, amount, "Transfer")
                .await
        } else {
            Err(WalletError::InvalidCurrency)
        }
    }

    /// Convert between currencies. Same-currency is a no-op. Cross-currency debits from_currency and credits to_currency at simulator rate.
    pub async fn convert(
        &self,
        player_id: Uuid,
        from_currency: &str,
        to_currency: &str,
        amount: i64,
    ) -> Result<i64, WalletError> {
        if from_currency == to_currency {
            return Ok(amount);
        }
        let out_amount =
            convert_amount(amount, from_currency, to_currency).ok_or(WalletError::InvalidCurrency)?;
        if out_amount == 0 {
            return Err(WalletError::ConvertedAmountTooSmall);
        }
        // Check balance for from_currency
        let from_balance = if from_currency == "USD" {
            let account = self.fkebank.get_by_owner("player", player_id).await?;
            account.balance
        } else {
            let key = player_crypto_vault_key(player_id, from_currency);
            let _ = self.crypto.register(&key, None, from_currency).await;
            self.crypto.get_balance(&key).await.unwrap_or(0)
        };
        if from_balance < amount {
            return Err(WalletError::InsufficientBalance);
        }
        // Debit from_currency
        if from_currency == "USD" {
            let account = self.fkebank.get_by_owner("player", player_id).await?;
            self.fkebank
                .debit(&account.key, amount, "Convert")
                .await?;
        } else {
            let key = player_crypto_vault_key(player_id, from_currency);
            self.crypto
                .debit(from_currency, &key, amount, "Convert")
                .await?;
        }
        // Credit to_currency
        if to_currency == "USD" {
            let account = self.fkebank.get_by_owner("player", player_id).await?;
            self.fkebank
                .credit(&account.key, out_amount, "Convert")
                .await?;
        } else {
            let key = player_crypto_vault_key(player_id, to_currency);
            let _ = self.crypto.register(&key, None, to_currency).await;
            self.crypto
                .credit(to_currency, &key, out_amount, "Convert", "system")
                .await?;
        }
        Ok(out_amount)
    }

    /// Returns keys: only USD (Fkebank) key for the player. Crypto has no owner in DB.
    pub async fn get_keys(&self, player_id: Uuid) -> Result<Vec<WalletKey>, WalletError> {
        let account = self.fkebank.get_by_owner("player", player_id).await?;
        Ok(vec![WalletKey {
            currency: "USD".to_string(),
            key_address: account.key,
        }])
    }

    /// Resolves Fkebank key to player_id if the account is player-owned.
    pub async fn find_player_by_address(&self, address: &str) -> Result<Option<Uuid>, WalletError> {
        let account = self.fkebank.get_by_key(address).await?;
        Ok(account.filter(|a| a.owner_type == "player").map(|a| a.owner_id))
    }
}

#[cfg(test)]
mod tests {
    use super::super::player_service::PlayerService;
    use super::super::test_pool;
    use super::*;

    fn unique_username() -> String {
        format!("wallet_test_{}", Uuid::new_v4())
    }

    async fn create_test_player(pool: &sqlx::PgPool) -> Uuid {
        let ps = PlayerService::new(pool.clone());
        let name = unique_username();
        let p = ps.create_player(&name, "pw").await.unwrap();
        p.id
    }

    #[tokio::test]
    async fn test_create_wallet_for_player_creates_account_and_usd_key() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        assert_eq!(balances.len(), 4);
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 0);
        let keys = ws.get_keys(player_id).await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].key_address.starts_with("fkebank-"));
    }

    #[tokio::test]
    async fn test_create_wallet_for_player_idempotent() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool);

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.create_wallet_for_player(player_id).await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        assert_eq!(balances.len(), 4);
        let keys = ws.get_keys(player_id).await.unwrap();
        assert_eq!(keys.len(), 1);
    }

    #[tokio::test]
    async fn test_create_wallet_for_vm_returns_key_and_token() {
        let pool = test_pool().await;
        let ws = WalletService::new(pool.clone());
        let vm_id = Uuid::new_v4();

        let (key, token) = ws.create_wallet_for_vm(vm_id).await.unwrap();
        assert!(key.starts_with("fkebank-"));
        assert!(!token.is_empty());
        assert!(token.len() >= 32);

        let (key2, token2) = ws.create_wallet_for_vm(vm_id).await.unwrap();
        assert_eq!(key, key2);
        assert!(!token2.is_empty());
    }

    #[tokio::test]
    async fn test_credit_increases_balance_and_creates_transaction() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "test credit").await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 10000);

        let txs = ws.get_transactions(player_id, "all").await.unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].tx_type, "credit");
        assert_eq!(txs[0].currency, "USD");
        assert_eq!(txs[0].amount, 10000);
    }

    #[tokio::test]
    async fn test_debit_decreases_balance() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 5000, "seed").await.unwrap();
        ws.debit(player_id, "USD", 2000, "withdraw").await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 3000);
    }

    #[tokio::test]
    async fn test_debit_insufficient_balance_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let res = ws.debit(player_id, "USD", 100, "any").await;
        assert!(matches!(res, Err(WalletError::InsufficientBalance)));
    }

    #[tokio::test]
    async fn test_get_transactions_filter_today() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 100, "today").await.unwrap();

        let txs = ws.get_transactions(player_id, "today").await.unwrap();
        assert!(!txs.is_empty());
        let txs_all = ws.get_transactions(player_id, "all").await.unwrap();
        assert_eq!(txs_all.len(), 1);
    }

    #[tokio::test]
    async fn test_transfer_to_external_address() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 5000, "seed").await.unwrap();

        ws.transfer_to_address(player_id, "external-fkebank-deadbeef", "USD", 1000)
            .await
            .unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 4000);

        let txs = ws.get_transactions(player_id, "all").await.unwrap();
        assert!(txs.iter().any(|t| t.tx_type == "transfer_out" && t.amount == 1000));
    }

    #[tokio::test]
    async fn test_transfer_between_players_atomic() {
        let pool = test_pool().await;
        let ps = PlayerService::new(pool.clone());
        let ws = WalletService::new(pool.clone());

        let name_a = unique_username();
        let name_b = unique_username();
        let p_a = ps.create_player(&name_a, "pw").await.unwrap();
        let p_b = ps.create_player(&name_b, "pw").await.unwrap();

        ws.create_wallet_for_player(p_a.id).await.unwrap();
        ws.create_wallet_for_player(p_b.id).await.unwrap();
        ws.credit(p_a.id, "USD", 10000, "seed").await.unwrap();

        let keys_b = ws.get_keys(p_b.id).await.unwrap();
        let addr_b = keys_b[0].key_address.clone();

        ws.transfer_to_address(p_a.id, &addr_b, "USD", 3000).await.unwrap();

        let bal_a = ws.get_balances(p_a.id).await.unwrap();
        let bal_b = ws.get_balances(p_b.id).await.unwrap();
        assert_eq!(bal_a.iter().find(|b| b.currency == "USD").unwrap().balance, 7000);
        assert_eq!(bal_b.iter().find(|b| b.currency == "USD").unwrap().balance, 3000);

        let txs_a = ws.get_transactions(p_a.id, "all").await.unwrap();
        let txs_b = ws.get_transactions(p_b.id, "all").await.unwrap();
        assert!(txs_a.iter().any(|t| t.tx_type == "transfer_out"));
        assert!(txs_b.iter().any(|t| t.tx_type == "transfer_in"));
    }

    #[tokio::test]
    async fn test_transfer_to_self_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 1000, "seed").await.unwrap();

        let keys = ws.get_keys(player_id).await.unwrap();
        let my_addr = keys[0].key_address.clone();

        let res = ws.transfer_to_address(player_id, &my_addr, "USD", 100).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_convert_same_currency_returns_amount() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let out = ws.convert(player_id, "USD", "USD", 10000).await.unwrap();
        assert_eq!(out, 10000);
    }

    #[tokio::test]
    async fn test_convert_cross_currency_succeeds() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "seed").await.unwrap();
        let out = ws.convert(player_id, "USD", "BTC", 10000).await.unwrap();
        assert_eq!(out, 40); // 10000 USD cents at 250 USD per BTC -> 40 BTC cents
        let balances = ws.get_balances(player_id).await.unwrap();
        assert_eq!(
            balances.iter().find(|b| b.currency == "USD").unwrap().balance,
            0
        );
        assert_eq!(
            balances.iter().find(|b| b.currency == "BTC").unwrap().balance,
            40
        );
    }

    #[tokio::test]
    async fn test_find_player_by_address() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let keys = ws.get_keys(player_id).await.unwrap();
        let usd_addr = keys[0].key_address.clone();

        let found = ws.find_player_by_address(&usd_addr).await.unwrap();
        assert_eq!(found, Some(player_id));
        assert!(ws.find_player_by_address("nonexistent-addr").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_key_formats() {
        let usd = generate_fkebank_key();
        assert!(usd.starts_with("fkebank-"));
        assert!(usd.len() >= 40);

        let btc = generate_btc_address();
        assert!(btc.starts_with("bc1q"));
        assert!(btc.len() >= 40);

        let eth = generate_eth_address();
        assert!(eth.starts_with("0x"));
        assert_eq!(eth.len(), 42);

        let sol = generate_sol_address();
        assert_eq!(sol.len(), 44);
    }

    #[tokio::test]
    async fn test_convert_amount_rates() {
        assert_eq!(convert_amount(100, "USD", "USD"), Some(100));
        assert_eq!(convert_amount(10000, "USD", "BTC"), Some(40));
        assert_eq!(convert_amount(100, "BTC", "USD"), Some(25000));
    }

    #[tokio::test]
    async fn test_get_balances_without_wallet_returns_zero_for_all_currencies() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool);

        let balances = ws.get_balances(player_id).await.unwrap();
        assert_eq!(balances.len(), 4);
        for b in &balances {
            assert_eq!(b.balance, 0);
        }
        let currencies: Vec<_> = balances.iter().map(|b| b.currency.as_str()).collect();
        assert!(currencies.contains(&"USD"));
        assert!(currencies.contains(&"BTC"));
        assert!(currencies.contains(&"ETH"));
        assert!(currencies.contains(&"SOL"));
    }

    #[tokio::test]
    async fn test_get_transactions_without_wallet_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool);

        let res = ws.get_transactions(player_id, "all").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_get_keys_without_wallet_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool);

        let res = ws.get_keys(player_id).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_credit_non_usd_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let res = ws.credit(player_id, "BTC", 100, "test").await;
        assert!(matches!(res, Err(WalletError::InvalidCurrency)));
    }

    #[tokio::test]
    async fn test_debit_non_usd_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 1000, "seed").await.unwrap();
        let res = ws.debit(player_id, "ETH", 100, "test").await;
        assert!(matches!(res, Err(WalletError::InvalidCurrency)));
    }

    #[tokio::test]
    async fn test_credit_without_wallet_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool);

        let res = ws.credit(player_id, "USD", 1000, "seed").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_transfer_invalid_currency_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 1000, "seed").await.unwrap();
        let res = ws.transfer_to_address(player_id, "fkebank-somekey", "XYZ", 100).await;
        assert!(matches!(res, Err(WalletError::InvalidCurrency)));
    }

    #[tokio::test]
    async fn test_transfer_btc_succeeds() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "seed").await.unwrap();
        ws.convert(player_id, "USD", "BTC", 10000).await.unwrap(); // 40 BTC cents
        let target_btc = generate_btc_address();
        ws.transfer_to_address(player_id, &target_btc, "BTC", 20).await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        assert_eq!(balances.iter().find(|b| b.currency == "BTC").unwrap().balance, 20);
        let recipient_balance = ws.crypto_service().get_balance(&target_btc).await.unwrap();
        assert_eq!(recipient_balance, 20);
    }

    #[tokio::test]
    async fn test_transfer_eth_succeeds() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "seed").await.unwrap();
        ws.convert(player_id, "USD", "ETH", 10000).await.unwrap();
        let target_eth = generate_eth_address();
        ws.transfer_to_address(player_id, &target_eth, "ETH", 100).await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let eth_bal = balances.iter().find(|b| b.currency == "ETH").unwrap().balance;
        assert_eq!(eth_bal, 400); // 500 - 100 at rate 20 USD/cent = 500 ETH cents, minus 100
        let recipient_balance = ws.crypto_service().get_balance(&target_eth).await.unwrap();
        assert_eq!(recipient_balance, 100);
    }

    #[tokio::test]
    async fn test_transfer_sol_succeeds() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "seed").await.unwrap();
        ws.convert(player_id, "USD", "SOL", 10000).await.unwrap();
        let target_sol = generate_sol_address();
        ws.transfer_to_address(player_id, &target_sol, "SOL", 200).await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let sol_bal = balances.iter().find(|b| b.currency == "SOL").unwrap().balance;
        assert_eq!(sol_bal, 9800); // 10000 SOL cents at rate 1, minus 200
        let recipient_balance = ws.crypto_service().get_balance(&target_sol).await.unwrap();
        assert_eq!(recipient_balance, 200);
    }

    #[tokio::test]
    async fn test_transfer_crypto_to_self_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 10000, "seed").await.unwrap();
        ws.convert(player_id, "USD", "BTC", 5000).await.unwrap();
        let vault_key = format!("player-vault-{}-BTC", player_id);
        let res = ws.transfer_to_address(player_id, &vault_key, "BTC", 10).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_transfer_all_currencies_between_players() {
        let pool = test_pool().await;
        let ps = PlayerService::new(pool.clone());
        let ws = WalletService::new(pool.clone());

        let p_a = ps.create_player(&unique_username(), "pw").await.unwrap();
        let p_b = ps.create_player(&unique_username(), "pw").await.unwrap();
        ws.create_wallet_for_player(p_a.id).await.unwrap();
        ws.create_wallet_for_player(p_b.id).await.unwrap();

        // Seed A with 110 USD; convert 50/30/30 to BTC/ETH/SOL so 10 USD remains
        ws.credit(p_a.id, "USD", 110_00, "seed").await.unwrap();
        ws.convert(p_a.id, "USD", "BTC", 50_00).await.unwrap();   // 20 BTC cents, 60 USD left
        ws.convert(p_a.id, "USD", "ETH", 30_00).await.unwrap();   // 150 ETH cents, 30 USD left
        ws.convert(p_a.id, "USD", "SOL", 20_00).await.unwrap();   // 2000 SOL cents, 10 USD left

        let keys_b = ws.get_keys(p_b.id).await.unwrap();
        let addr_usd_b = &keys_b[0].key_address;
        let addr_btc_b = generate_btc_address();
        let addr_eth_b = generate_eth_address();
        let addr_sol_b = generate_sol_address();

        // USD: A -> B (internal Fkebank)
        ws.transfer_to_address(p_a.id, addr_usd_b, "USD", 10_00).await.unwrap();
        // BTC: A -> B address
        ws.transfer_to_address(p_a.id, &addr_btc_b, "BTC", 5).await.unwrap();
        // ETH: A -> B address
        ws.transfer_to_address(p_a.id, &addr_eth_b, "ETH", 50).await.unwrap();
        // SOL: A -> B address
        ws.transfer_to_address(p_a.id, &addr_sol_b, "SOL", 50).await.unwrap();

        let bal_a = ws.get_balances(p_a.id).await.unwrap();
        let bal_b = ws.get_balances(p_b.id).await.unwrap();

        assert_eq!(bal_a.iter().find(|b| b.currency == "USD").unwrap().balance, 0);
        assert_eq!(bal_b.iter().find(|b| b.currency == "USD").unwrap().balance, 10_00);

        assert_eq!(ws.crypto_service().get_balance(&addr_btc_b).await.unwrap(), 5);
        assert_eq!(ws.crypto_service().get_balance(&addr_eth_b).await.unwrap(), 50);
        assert_eq!(ws.crypto_service().get_balance(&addr_sol_b).await.unwrap(), 50);
    }

    #[tokio::test]
    async fn test_find_player_by_address_vm_account_returns_none() {
        let pool = test_pool().await;
        let ws = WalletService::new(pool.clone());
        let vm_id = Uuid::new_v4();

        let (key, _) = ws.create_wallet_for_vm(vm_id).await.unwrap();
        let found = ws.find_player_by_address(&key).await.unwrap();
        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn test_get_transactions_empty_before_credit() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let txs = ws.get_transactions(player_id, "all").await.unwrap();
        assert!(txs.is_empty());
    }

    #[tokio::test]
    async fn test_convert_same_currency_zero_amount() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        let out = ws.convert(player_id, "USD", "USD", 0).await.unwrap();
        assert_eq!(out, 0);
    }

    #[tokio::test]
    async fn test_multiple_credits_and_debits_accumulate() {
        let pool = test_pool().await;
        let player_id = create_test_player(&pool).await;
        let ws = WalletService::new(pool.clone());

        ws.create_wallet_for_player(player_id).await.unwrap();
        ws.credit(player_id, "USD", 1000, "a").await.unwrap();
        ws.credit(player_id, "USD", 2000, "b").await.unwrap();
        ws.debit(player_id, "USD", 500, "c").await.unwrap();

        let balances = ws.get_balances(player_id).await.unwrap();
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 2500);

        let txs = ws.get_transactions(player_id, "all").await.unwrap();
        assert_eq!(txs.len(), 3);
    }

    #[tokio::test]
    async fn test_transfer_in_records_counterpart() {
        let pool = test_pool().await;
        let ps = PlayerService::new(pool.clone());
        let ws = WalletService::new(pool.clone());

        let p_a = ps.create_player(&unique_username(), "pw").await.unwrap();
        let p_b = ps.create_player(&unique_username(), "pw").await.unwrap();
        ws.create_wallet_for_player(p_a.id).await.unwrap();
        ws.create_wallet_for_player(p_b.id).await.unwrap();
        ws.credit(p_a.id, "USD", 5000, "seed").await.unwrap();

        let keys_b = ws.get_keys(p_b.id).await.unwrap();
        ws.transfer_to_address(p_a.id, &keys_b[0].key_address, "USD", 1000).await.unwrap();

        let txs_b = ws.get_transactions(p_b.id, "all").await.unwrap();
        let transfer_in = txs_b.iter().find(|t| t.tx_type == "transfer_in").unwrap();
        assert_eq!(transfer_in.amount, 1000);
        assert!(transfer_in.counterpart_address.as_deref().unwrap_or("").starts_with("fkebank-"));
    }
}
