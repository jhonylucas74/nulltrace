#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum WalletError {
    Db(sqlx::Error),
    InsufficientBalance,
    InvalidCurrency,
    CardLimitExceeded,
}

impl From<sqlx::Error> for WalletError {
    fn from(e: sqlx::Error) -> Self {
        WalletError::Db(e)
    }
}

impl std::fmt::Display for WalletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletError::Db(e) => write!(f, "Database error: {}", e),
            WalletError::InsufficientBalance => write!(f, "Insufficient balance"),
            WalletError::InvalidCurrency => write!(f, "Invalid currency"),
            WalletError::CardLimitExceeded => write!(f, "Card credit limit exceeded"),
        }
    }
}

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct WalletBalance {
    pub currency: String,
    pub balance: i64,
}

#[derive(Debug, Clone, FromRow)]
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

#[derive(Debug, Clone, FromRow)]
pub struct WalletKey {
    pub currency: String,
    pub key_address: String,
}

const CURRENCIES: &[&str] = &["USD", "BTC", "ETH", "SOL"];

// ── Address generators ────────────────────────────────────────────────────────

/// Fkebank PIX-style key: fkebank-{32 hex chars}
pub fn generate_fkebank_key() -> String {
    format!("fkebank-{}", Uuid::new_v4().simple())
}

/// ETH-style address: 0x{40 lowercase hex chars}
pub fn generate_eth_address() -> String {
    let hex1 = format!("{}", Uuid::new_v4().simple());
    let hex2 = format!("{}", Uuid::new_v4().simple());
    let combined = format!("{}{}", hex1, hex2);
    format!("0x{}", &combined[..40])
}

/// BTC bech32-style address: bc1q{38 bech32 chars}
pub fn generate_btc_address() -> String {
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let bytes: Vec<u8> = Uuid::new_v4()
        .as_bytes()
        .iter()
        .chain(Uuid::new_v4().as_bytes().iter())
        .chain(Uuid::new_v4().as_bytes().iter())
        .copied()
        .collect();
    let chars: String = bytes[..38]
        .iter()
        .map(|b| CHARSET[(*b as usize) % CHARSET.len()] as char)
        .collect();
    format!("bc1q{}", chars)
}

/// SOL base58-style address: 44 chars
pub fn generate_sol_address() -> String {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let bytes: Vec<u8> = Uuid::new_v4()
        .as_bytes()
        .iter()
        .chain(Uuid::new_v4().as_bytes().iter())
        .chain(Uuid::new_v4().as_bytes().iter())
        .copied()
        .collect();
    bytes[..44]
        .iter()
        .map(|b| BASE58[(*b as usize) % BASE58.len()] as char)
        .collect()
}

fn generate_key_for_currency(currency: &str) -> String {
    match currency {
        "USD" => generate_fkebank_key(),
        "BTC" => generate_btc_address(),
        "ETH" => generate_eth_address(),
        "SOL" => generate_sol_address(),
        _ => format!("key-{}", Uuid::new_v4()),
    }
}

// ── Conversion rates ──────────────────────────────────────────────────────────
//
// In-game rates (USD value of 1 DB-cent of each currency):
//   USD: 1 cent   = $0.01   → factor = 1.0
//   BTC: 0.01 BTC = $250.00 → factor = 250.0
//   ETH: 0.01 ETH = $20.00  → factor = 20.0
//   SOL: 0.01 SOL = $1.00   → factor = 1.0
//
// Formula: out_amount = in_amount * in_factor / out_factor

fn usd_factor_per_cent(currency: &str) -> Option<f64> {
    match currency {
        "USD" => Some(1.0),
        "BTC" => Some(250.0),
        "ETH" => Some(20.0),
        "SOL" => Some(1.0),
        _ => None,
    }
}

/// Converts `amount` DB-cents from `from` currency into DB-cents of `to` currency.
/// Returns None if either currency is unknown.
pub fn convert_amount(amount: i64, from: &str, to: &str) -> Option<i64> {
    let in_factor = usd_factor_per_cent(from)?;
    let out_factor = usd_factor_per_cent(to)?;
    if out_factor == 0.0 {
        return None;
    }
    Some((amount as f64 * in_factor / out_factor).floor() as i64)
}

// ── Service ───────────────────────────────────────────────────────────────────

pub struct WalletService {
    pool: PgPool,
}

impl WalletService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates wallet accounts + receive keys for all currencies.
    /// Idempotent — safe to call multiple times (ON CONFLICT DO NOTHING).
    pub async fn create_wallet_for_player(&self, player_id: Uuid) -> Result<(), WalletError> {
        for currency in CURRENCIES {
            sqlx::query(
                r#"
                INSERT INTO wallet_accounts (id, player_id, currency, balance)
                VALUES ($1, $2, $3, 0)
                ON CONFLICT (player_id, currency) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(player_id)
            .bind(currency)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO wallet_keys (id, player_id, currency, key_address)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (player_id, currency) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(player_id)
            .bind(currency)
            .bind(generate_key_for_currency(currency))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Returns all wallet balances for a player (one row per currency).
    pub async fn get_balances(&self, player_id: Uuid) -> Result<Vec<WalletBalance>, WalletError> {
        let rows = sqlx::query_as::<_, WalletBalance>(
            "SELECT currency, balance FROM wallet_accounts WHERE player_id = $1 ORDER BY currency",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Returns transactions for a player, optionally filtered by time range.
    /// filter: "today" | "7d" | "30d" | "all"
    pub async fn get_transactions(
        &self,
        player_id: Uuid,
        filter: &str,
    ) -> Result<Vec<WalletTransaction>, WalletError> {
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
            sqlx::query_as::<_, WalletTransaction>(
                r#"
                SELECT id, player_id, tx_type, currency, amount, fee, description,
                       counterpart_address, counterpart_player_id, related_transaction_id, created_at
                FROM wallet_transactions
                WHERE player_id = $1 AND created_at >= $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(player_id)
            .bind(after)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, WalletTransaction>(
                r#"
                SELECT id, player_id, tx_type, currency, amount, fee, description,
                       counterpart_address, counterpart_player_id, related_transaction_id, created_at
                FROM wallet_transactions
                WHERE player_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(player_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    /// Adds `amount` to the player's balance and records a credit transaction.
    pub async fn credit(
        &self,
        player_id: Uuid,
        currency: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        sqlx::query(
            "UPDATE wallet_accounts SET balance = balance + $1, updated_at = now() \
             WHERE player_id = $2 AND currency = $3",
        )
        .bind(amount)
        .bind(player_id)
        .bind(currency)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, player_id, tx_type, currency, amount, fee, description)
            VALUES ($1, $2, 'credit', $3, $4, 0, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(currency)
        .bind(amount)
        .bind(description)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Removes `amount` from the player's balance. Fails with InsufficientBalance if balance < amount.
    pub async fn debit(
        &self,
        player_id: Uuid,
        currency: &str,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        let updated = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE wallet_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE player_id = $2 AND currency = $3 AND balance >= $1
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(player_id)
        .bind(currency)
        .fetch_optional(&self.pool)
        .await?;

        if updated.is_none() {
            return Err(WalletError::InsufficientBalance);
        }

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, player_id, tx_type, currency, amount, fee, description)
            VALUES ($1, $2, 'debit', $3, $4, 0, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(currency)
        .bind(amount)
        .bind(description)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Transfer to an address. If the address belongs to another player in this game,
    /// it becomes an internal (atomic) transfer. Otherwise it's an external debit.
    pub async fn transfer_to_address(
        &self,
        player_id: Uuid,
        target_address: &str,
        currency: &str,
        amount: i64,
    ) -> Result<(), WalletError> {
        let target_player = self.find_player_by_address(target_address).await?;

        if let Some(target_id) = target_player {
            if target_id == player_id {
                // Can't transfer to yourself
                return Err(WalletError::InvalidCurrency);
            }
            self.transfer_between_players_inner(player_id, target_id, target_address, currency, amount)
                .await
        } else {
            // External transfer
            let updated = sqlx::query_as::<_, (i64,)>(
                r#"
                UPDATE wallet_accounts
                SET balance = balance - $1, updated_at = now()
                WHERE player_id = $2 AND currency = $3 AND balance >= $1
                RETURNING balance
                "#,
            )
            .bind(amount)
            .bind(player_id)
            .bind(currency)
            .fetch_optional(&self.pool)
            .await?;

            if updated.is_none() {
                return Err(WalletError::InsufficientBalance);
            }

            sqlx::query(
                r#"
                INSERT INTO wallet_transactions
                    (id, player_id, tx_type, currency, amount, fee, description, counterpart_address)
                VALUES ($1, $2, 'transfer_out', $3, $4, 0, 'External transfer', $5)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(player_id)
            .bind(currency)
            .bind(amount)
            .bind(target_address)
            .execute(&self.pool)
            .await?;

            Ok(())
        }
    }

    /// Atomic transfer between two players using a single SQL transaction.
    async fn transfer_between_players_inner(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        target_address: &str,
        currency: &str,
        amount: i64,
    ) -> Result<(), WalletError> {
        let mut tx = self.pool.begin().await?;

        // Debit sender atomically
        let deducted = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE wallet_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE player_id = $2 AND currency = $3 AND balance >= $1
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(from_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?;

        if deducted.is_none() {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }

        // Credit receiver
        sqlx::query(
            "UPDATE wallet_accounts SET balance = balance + $1, updated_at = now() \
             WHERE player_id = $2 AND currency = $3",
        )
        .bind(amount)
        .bind(to_id)
        .bind(currency)
        .execute(&mut *tx)
        .await?;

        // Retrieve sender's address for the receiver's ledger entry
        let from_addr: String = sqlx::query_as::<_, (String,)>(
            "SELECT key_address FROM wallet_keys WHERE player_id = $1 AND currency = $2",
        )
        .bind(from_id)
        .bind(currency)
        .fetch_optional(&mut *tx)
        .await?
        .map(|r| r.0)
        .unwrap_or_default();

        let sender_tx_id = Uuid::new_v4();
        let receiver_tx_id = Uuid::new_v4();

        // Sender ledger
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions
                (id, player_id, tx_type, currency, amount, fee, description,
                 counterpart_address, counterpart_player_id, related_transaction_id)
            VALUES ($1, $2, 'transfer_out', $3, $4, 0, 'Transfer sent', $5, $6, $7)
            "#,
        )
        .bind(sender_tx_id)
        .bind(from_id)
        .bind(currency)
        .bind(amount)
        .bind(target_address)
        .bind(to_id)
        .bind(receiver_tx_id)
        .execute(&mut *tx)
        .await?;

        // Receiver ledger
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions
                (id, player_id, tx_type, currency, amount, fee, description,
                 counterpart_address, counterpart_player_id, related_transaction_id)
            VALUES ($1, $2, 'transfer_in', $3, $4, 0, 'Transfer received', $5, $6, $7)
            "#,
        )
        .bind(receiver_tx_id)
        .bind(to_id)
        .bind(currency)
        .bind(amount)
        .bind(from_addr)
        .bind(from_id)
        .bind(sender_tx_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Convert `amount` DB-cents from `from_currency` to `to_currency` using in-game rates.
    /// Returns the resulting amount in `to_currency` DB-cents.
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
            return Err(WalletError::InsufficientBalance);
        }

        let mut tx = self.pool.begin().await?;

        let deducted = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE wallet_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE player_id = $2 AND currency = $3 AND balance >= $1
            RETURNING balance
            "#,
        )
        .bind(amount)
        .bind(player_id)
        .bind(from_currency)
        .fetch_optional(&mut *tx)
        .await?;

        if deducted.is_none() {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }

        sqlx::query(
            "UPDATE wallet_accounts SET balance = balance + $1, updated_at = now() \
             WHERE player_id = $2 AND currency = $3",
        )
        .bind(out_amount)
        .bind(player_id)
        .bind(to_currency)
        .execute(&mut *tx)
        .await?;

        let desc = format!("Convert {} {} → {}", amount, from_currency, to_currency);

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, player_id, tx_type, currency, amount, fee, description)
            VALUES ($1, $2, 'convert', $3, $4, 0, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(from_currency)
        .bind(amount)
        .bind(&desc)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, player_id, tx_type, currency, amount, fee, description)
            VALUES ($1, $2, 'convert', $3, $4, 0, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(to_currency)
        .bind(out_amount)
        .bind(&desc)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(out_amount)
    }

    /// Returns all receive addresses for a player (one per currency).
    pub async fn get_keys(&self, player_id: Uuid) -> Result<Vec<WalletKey>, WalletError> {
        let rows = sqlx::query_as::<_, WalletKey>(
            "SELECT currency, key_address FROM wallet_keys WHERE player_id = $1 ORDER BY currency",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Returns the player_id that owns the given address, if any.
    pub async fn find_player_by_address(&self, address: &str) -> Result<Option<Uuid>, WalletError> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT player_id FROM wallet_keys WHERE key_address = $1",
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.0))
    }
}
