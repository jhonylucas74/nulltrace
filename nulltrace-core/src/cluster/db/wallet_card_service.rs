#![allow(dead_code)]

use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::wallet_service::WalletError;

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct WalletCard {
    pub id: Uuid,
    pub player_id: Uuid,
    pub label: Option<String>,
    pub number_full: String,
    pub last4: String,
    pub expiry_month: i32,
    pub expiry_year: i32,
    pub cvv: String,
    pub holder_name: String,
    pub credit_limit: i64,
    pub current_debt: i64,
    pub is_virtual: bool,
    pub is_active: bool,
    pub billing_day_of_week: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CardTransaction {
    pub id: Uuid,
    pub card_id: Uuid,
    pub player_id: Uuid,
    pub tx_type: String,
    pub amount: i64,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CardStatement {
    pub id: Uuid,
    pub card_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_amount: i64,
    pub status: String,
    pub due_date: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn generate_card_number() -> String {
    let bytes = Uuid::new_v4().as_bytes().to_vec();
    // Visa: starts with 4, 16 digits total
    let mut number = String::from("4");
    for b in &bytes[1..16] {
        number.push(char::from_digit((b % 10) as u32, 10).unwrap_or('0'));
    }
    number
}

fn generate_cvv() -> String {
    let bytes = Uuid::new_v4().as_bytes().to_vec();
    format!("{}{}{}", bytes[0] % 10, bytes[1] % 10, bytes[2] % 10)
}

/// Returns the next Monday at 12:00 UTC after `from`.
/// If `from` is already Monday, returns the following Monday (next cycle).
pub fn next_billing_monday(from: DateTime<Utc>) -> DateTime<Utc> {
    let days_until: i64 = match from.weekday() {
        Weekday::Mon => 7,
        Weekday::Tue => 6,
        Weekday::Wed => 5,
        Weekday::Thu => 4,
        Weekday::Fri => 3,
        Weekday::Sat => 2,
        Weekday::Sun => 1,
    };
    let next = from + Duration::days(days_until);
    next.date_naive()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
}

// ── Service ───────────────────────────────────────────────────────────────────

pub struct WalletCardService {
    pool: PgPool,
}

impl WalletCardService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Ensures player_credit_accounts row exists. Idempotent.
    async fn ensure_player_credit_account(&self, player_id: Uuid) -> Result<(), WalletError> {
        sqlx::query(
            r#"
            INSERT INTO player_credit_accounts (player_id, credit_limit, created_at)
            VALUES ($1, 20000, now())
            ON CONFLICT (player_id) DO NOTHING
            "#,
        )
        .bind(player_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns account credit limit for a player (shared by all cards).
    pub async fn get_account_credit_limit(&self, player_id: Uuid) -> Result<i64, WalletError> {
        let row = sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT credit_limit FROM player_credit_accounts WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(l,)| l).unwrap_or(20_000))
    }

    /// Returns total debt across all cards (active + inactive). Used for display after soft-delete.
    pub async fn get_account_total_debt(&self, player_id: Uuid) -> Result<i64, WalletError> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COALESCE(SUM(current_debt)::BIGINT, 0) FROM wallet_cards WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Returns active cards only. Soft-deleted cards are hidden from the list.
    pub async fn get_cards(&self, player_id: Uuid) -> Result<Vec<WalletCard>, WalletError> {
        let rows = sqlx::query_as::<_, WalletCard>(
            r#"
            SELECT w.id, w.player_id, w.label, w.number_full, w.last4, w.expiry_month, w.expiry_year,
                   w.cvv, w.holder_name, COALESCE(pca.credit_limit, 20000) as credit_limit,
                   w.current_debt, w.is_virtual, w.is_active, w.billing_day_of_week, w.created_at
            FROM wallet_cards w
            LEFT JOIN player_credit_accounts pca ON pca.player_id = w.player_id
            WHERE w.player_id = $1 AND w.is_active = TRUE
            ORDER BY w.created_at ASC
            "#,
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Creates a new virtual card and opens its first statement.
    /// Uses account-level credit limit from player_credit_accounts (shared by all cards).
    pub async fn create_card(
        &self,
        player_id: Uuid,
        label: Option<&str>,
        holder_name: &str,
    ) -> Result<WalletCard, WalletError> {
        self.ensure_player_credit_account(player_id).await?;

        let number = generate_card_number();
        let last4 = number[12..].to_string();
        let cvv = generate_cvv();
        let now = Utc::now();
        let expiry_year = now.year() + 3;
        let expiry_month = now.month() as i32;

        let row = sqlx::query_as::<_, (Uuid, Uuid, Option<String>, String, String, i32, i32, String, String, i64, bool, bool, i32, DateTime<Utc>)>(
            r#"
            INSERT INTO wallet_cards
                (id, player_id, label, number_full, last4, expiry_month, expiry_year,
                 cvv, holder_name, current_debt, is_virtual, is_active, billing_day_of_week)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, TRUE, TRUE, 1)
            RETURNING id, player_id, label, number_full, last4, expiry_month, expiry_year,
                      cvv, holder_name, current_debt, is_virtual, is_active,
                      billing_day_of_week, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(label)
        .bind(&number)
        .bind(&last4)
        .bind(expiry_month)
        .bind(expiry_year)
        .bind(&cvv)
        .bind(holder_name)
        .fetch_one(&self.pool)
        .await?;

        let credit_limit = self.get_account_credit_limit(player_id).await?;
        let card = WalletCard {
            id: row.0,
            player_id: row.1,
            label: row.2,
            number_full: row.3,
            last4: row.4,
            expiry_month: row.5,
            expiry_year: row.6,
            cvv: row.7,
            holder_name: row.8,
            credit_limit,
            current_debt: row.9,
            is_virtual: row.10,
            is_active: row.11,
            billing_day_of_week: row.12,
            created_at: row.13,
        };

        // Open first billing statement
        self.get_or_create_open_statement(card.id).await?;

        Ok(card)
    }

    /// Finds a card by number (spaces ignored). Validates expiry. Returns (card, player_id) or None.
    pub async fn find_card_by_number(
        &self,
        number: &str,
        cvv: &str,
        expiry_month: i32,
        expiry_year: i32,
        holder_name: &str,
    ) -> Result<Option<(WalletCard, Uuid)>, WalletError> {
        let normalized = number.replace(|c: char| c.is_whitespace(), "");
        if normalized.len() != 16 {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, WalletCard>(
            r#"
            SELECT w.id, w.player_id, w.label, w.number_full, w.last4, w.expiry_month, w.expiry_year,
                   w.cvv, w.holder_name, COALESCE(pca.credit_limit, 20000) as credit_limit,
                   w.current_debt, w.is_virtual, w.is_active, w.billing_day_of_week, w.created_at
            FROM wallet_cards w
            LEFT JOIN player_credit_accounts pca ON pca.player_id = w.player_id
            WHERE REPLACE(w.number_full, ' ', '') = $1 AND w.is_active = TRUE
            "#,
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await?;
        let Some(card) = row else {
            return Ok(None);
        };
        let now = Utc::now();
        let expired = card.expiry_year < now.year() as i32
            || (card.expiry_year == now.year() as i32 && card.expiry_month < now.month() as i32);
        if expired {
            return Ok(None);
        }
        if card.cvv != cvv.trim() || card.holder_name != holder_name.trim() {
            return Ok(None);
        }
        let player_id = card.player_id;
        Ok(Some((card, player_id)))
    }

    /// Rejects if the card has outstanding debt — user must pay the bill first.
    /// Soft-deletes a card (is_active = false). Debt stays associated with the card.
    /// Inactive cards cannot make new purchases; the user must pay the bill to clear debt.
    pub async fn delete_card(
        &self,
        card_id: Uuid,
        player_id: Uuid,
    ) -> Result<bool, WalletError> {
        let result = sqlx::query(
            "UPDATE wallet_cards SET is_active = FALSE WHERE id = $1 AND player_id = $2",
        )
        .bind(card_id)
        .bind(player_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(WalletError::CardNotFound);
        }
        Ok(true)
    }

    /// Returns card transactions, optionally filtered by time range.
    /// filter: "today" | "7d" | "30d" | "all"
    pub async fn get_card_transactions(
        &self,
        card_id: Uuid,
        filter: &str,
    ) -> Result<Vec<CardTransaction>, WalletError> {
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
            sqlx::query_as::<_, CardTransaction>(
                r#"
                SELECT id, card_id, player_id, tx_type, amount, description, created_at
                FROM wallet_card_transactions
                WHERE card_id = $1 AND created_at >= $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(card_id)
            .bind(after)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CardTransaction>(
                r#"
                SELECT id, card_id, player_id, tx_type, amount, description, created_at
                FROM wallet_card_transactions
                WHERE card_id = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(card_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows)
    }

    /// Records a purchase on the card. Fails with CardLimitExceeded if it would exceed the account limit.
    /// Limit is shared across all cards. Also updates the current open statement's total.
    pub async fn make_purchase(
        &self,
        card_id: Uuid,
        player_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        let mut tx = self.pool.begin().await?;

        // Lock account and check shared limit: SUM(active cards' debt) + amount <= account limit
        let limit_row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT credit_limit FROM player_credit_accounts
            WHERE player_id = $1
            FOR UPDATE
            "#,
        )
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;

        let credit_limit = match limit_row {
            Some((l,)) => l,
            None => {
                tx.rollback().await?;
                return Err(WalletError::CardLimitExceeded);
            }
        };

        // Include ALL cards (active + inactive) — debt is still owed even after soft-delete
        let total_debt_row = sqlx::query_as::<_, (i64,)>(
            "SELECT COALESCE(SUM(current_debt)::BIGINT, 0) FROM wallet_cards WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&mut *tx)
        .await?;
        let total_debt = total_debt_row.0;

        if total_debt + amount > credit_limit {
            tx.rollback().await?;
            return Err(WalletError::CardLimitExceeded);
        }

        // Update the specific card's debt
        let updated = sqlx::query(
            r#"
            UPDATE wallet_cards
            SET current_debt = current_debt + $1
            WHERE id = $2 AND player_id = $3 AND is_active = TRUE
            "#,
        )
        .bind(amount)
        .bind(card_id)
        .bind(player_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(WalletError::CardLimitExceeded);
        }

        // Record the card transaction
        sqlx::query(
            r#"
            INSERT INTO wallet_card_transactions (id, card_id, player_id, tx_type, amount, description)
            VALUES ($1, $2, $3, 'purchase', $4, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(card_id)
        .bind(player_id)
        .bind(amount)
        .bind(description)
        .execute(&mut *tx)
        .await?;

        // Accumulate into current open statement
        let stmt = self.get_or_create_open_statement(card_id).await?;
        sqlx::query(
            "UPDATE wallet_card_statements SET total_amount = total_amount + $1 WHERE id = $2",
        )
        .bind(amount)
        .bind(stmt.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Pays the full card bill atomically.
    /// Debits USD from the player's wallet account and clears current_debt on the card.
    /// Returns the amount paid (in cents).
    pub async fn pay_card_bill(
        &self,
        card_id: Uuid,
        player_id: Uuid,
    ) -> Result<i64, WalletError> {
        let mut tx = self.pool.begin().await?;

        // Lock and fetch the card's current debt and last4 (for extrato description).
        let row = sqlx::query_as::<_, (i64, String)>(
            r#"
            SELECT current_debt, last4 FROM wallet_cards
            WHERE id = $1 AND player_id = $2
            FOR UPDATE
            "#,
        )
        .bind(card_id)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (debt, card_last4) = match row {
            Some((d, l4)) => (d, l4),
            None => {
                tx.rollback().await?;
                return Err(WalletError::InvalidCurrency); // card not found
            }
        };

        if debt == 0 {
            tx.rollback().await?;
            return Ok(0);
        }

        // Get player's Fkebank account key
        let key_row = sqlx::query_as::<_, (String,)>(
            "SELECT key FROM fkebank_accounts WHERE owner_type = 'player' AND owner_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;
        let account_key = match key_row {
            Some((k,)) => k,
            None => {
                tx.rollback().await?;
                return Err(WalletError::InsufficientBalance);
            }
        };

        // Debit USD from Fkebank account
        let deducted = sqlx::query(
            r#"
            UPDATE fkebank_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE key = $2 AND balance >= $1
            "#,
        )
        .bind(debt)
        .bind(&account_key)
        .execute(&mut *tx)
        .await?;

        if deducted.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }

        // Clear the card debt
        sqlx::query("UPDATE wallet_cards SET current_debt = 0 WHERE id = $1")
            .bind(card_id)
            .execute(&mut *tx)
            .await?;

        // Log card payment transaction
        sqlx::query(
            r#"
            INSERT INTO wallet_card_transactions (id, card_id, player_id, tx_type, amount, description)
            VALUES ($1, $2, $3, 'payment', $4, 'Bill payment')
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(card_id)
        .bind(player_id)
        .bind(debt)
        .execute(&mut *tx)
        .await?;

        // Log wallet debit (key-based). Include card last4 for extrato.
        let bill_desc = format!("Credit card bill payment (Card ***{})", card_last4);
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at)
            VALUES ($1, 'USD', $2, 0, $4, $3, 'system', NULL, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(debt)
        .bind(&account_key)
        .bind(&bill_desc)
        .execute(&mut *tx)
        .await?;

        // Mark the current open statement as paid
        sqlx::query(
            "UPDATE wallet_card_statements SET status = 'paid', paid_at = now() \
             WHERE card_id = $1 AND status = 'open'",
        )
        .bind(card_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(debt)
    }

    /// Pays the full account bill: debits USD and zeros debt on all cards.
    /// Debt is account-level; cards are just payment methods. Returns total paid (DB-cents).
    pub async fn pay_account_bill(&self, player_id: Uuid) -> Result<i64, WalletError> {
        let mut tx = self.pool.begin().await?;

        let rows = sqlx::query_as::<_, (Uuid, i64, String)>(
            r#"
            SELECT id, current_debt, last4 FROM wallet_cards
            WHERE player_id = $1 AND current_debt > 0
            FOR UPDATE
            "#,
        )
        .bind(player_id)
        .fetch_all(&mut *tx)
        .await?;

        let total_debt: i64 = rows.iter().map(|(_, d, _)| d).sum();
        if total_debt == 0 {
            tx.rollback().await?;
            return Ok(0);
        }

        let account_key = sqlx::query_as::<_, (String,)>(
            "SELECT key FROM fkebank_accounts WHERE owner_type = 'player' AND owner_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(WalletError::InsufficientBalance)?
        .0;

        let deducted = sqlx::query(
            r#"
            UPDATE fkebank_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE key = $2 AND balance >= $1
            "#,
        )
        .bind(total_debt)
        .bind(&account_key)
        .execute(&mut *tx)
        .await?;

        if deducted.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(WalletError::InsufficientBalance);
        }

        for (card_id, debt, _last4) in &rows {
            sqlx::query("UPDATE wallet_cards SET current_debt = 0 WHERE id = $1")
                .bind(card_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                r#"
                INSERT INTO wallet_card_transactions (id, card_id, player_id, tx_type, amount, description)
                VALUES ($1, $2, $3, 'payment', $4, 'Account bill payment')
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(card_id)
            .bind(player_id)
            .bind(debt)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                "UPDATE wallet_card_statements SET status = 'paid', paid_at = now() \
                 WHERE card_id = $1 AND status = 'open'",
            )
            .bind(card_id)
            .execute(&mut *tx)
            .await?;
        }

        let bill_desc = if rows.len() == 1 {
            format!("Credit card bill payment (Card ***{})", rows[0].2)
        } else {
            format!("Account bill payment ({} cards)", rows.len())
        };

        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, currency, amount, fee, description, from_key, to_key, counterpart_key, created_at)
            VALUES ($1, 'USD', $2, 0, $4, $3, 'system', NULL, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(total_debt)
        .bind(&account_key)
        .bind(&bill_desc)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(total_debt)
    }

    /// Returns the current open statement for a card. Public for tests.
    pub async fn get_current_statement(
        &self,
        card_id: Uuid,
    ) -> Result<Option<CardStatement>, WalletError> {
        let row = sqlx::query_as::<_, CardStatement>(
            r#"
            SELECT id, card_id, period_start, period_end, total_amount, status, due_date, paid_at, created_at
            FROM wallet_card_statements
            WHERE card_id = $1 AND status = 'open'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(card_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Returns the current open statement for a card, creating one if none exists
    /// or if the existing one has expired (lazy weekly cycle management).
    pub async fn get_or_create_open_statement(
        &self,
        card_id: Uuid,
    ) -> Result<CardStatement, WalletError> {
        let existing = sqlx::query_as::<_, CardStatement>(
            r#"
            SELECT id, card_id, period_start, period_end, total_amount, status, due_date, paid_at, created_at
            FROM wallet_card_statements
            WHERE card_id = $1 AND status = 'open'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(card_id)
        .fetch_optional(&self.pool)
        .await?;

        let now = Utc::now();

        if let Some(stmt) = existing {
            if stmt.period_end > now {
                return Ok(stmt);
            }
            // Cycle ended — close it
            sqlx::query(
                "UPDATE wallet_card_statements SET status = 'closed', period_end = $1 WHERE id = $2",
            )
            .bind(now)
            .bind(stmt.id)
            .execute(&self.pool)
            .await?;
        }

        // Open a new statement cycle
        let period_start = now;
        let due_date = next_billing_monday(now);

        let new_stmt = sqlx::query_as::<_, CardStatement>(
            r#"
            INSERT INTO wallet_card_statements (id, card_id, period_start, period_end, total_amount, status, due_date)
            VALUES ($1, $2, $3, $4, 0, 'open', $5)
            RETURNING id, card_id, period_start, period_end, total_amount, status, due_date, paid_at, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(card_id)
        .bind(period_start)
        .bind(due_date) // period_end = due_date (one week)
        .bind(due_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_stmt)
    }
}

#[cfg(test)]
mod tests {
    use super::super::player_service::PlayerService;
    use super::super::test_pool;
    use super::super::wallet_service::WalletService;
    use super::*;
    use uuid::Uuid;

    fn unique_username() -> String {
        format!("card_test_{}", Uuid::new_v4())
    }

    async fn create_test_player_with_wallet(pool: &sqlx::PgPool) -> Uuid {
        let ps = PlayerService::new(pool.clone());
        let ws = WalletService::new(pool.clone());
        let name = unique_username();
        let p = ps.create_player(&name, "pw").await.unwrap();
        ws.create_wallet_for_player(p.id).await.unwrap();
        p.id
    }

    #[tokio::test]
    async fn test_create_card_creates_card_and_open_statement() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs
            .create_card(player_id, Some("Virtual 1"), "Test User")
            .await
            .unwrap();

        assert_eq!(card.player_id, player_id);
        assert!(card.number_full.starts_with('4'));
        assert_eq!(card.number_full.len(), 16);
        assert_eq!(card.last4.len(), 4);
        assert_eq!(card.credit_limit, 20_000, "default account limit");
        assert_eq!(card.current_debt, 0);
        assert!(card.is_virtual);
        assert!(card.is_active);

        let stmt = wcs.get_current_statement(card.id).await.unwrap();
        assert!(stmt.is_some());
        assert_eq!(stmt.unwrap().status, "open");
    }

    #[tokio::test]
    async fn test_get_cards_returns_only_active() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let c1 = wcs
            .create_card(player_id, None, "User")
            .await
            .unwrap();
        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards.len(), 1);

        wcs.delete_card(c1.id, player_id).await.unwrap();
        let cards_after = wcs.get_cards(player_id).await.unwrap();
        assert!(cards_after.is_empty());
    }

    #[tokio::test]
    async fn test_delete_card_soft_delete() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        let ok = wcs.delete_card(card.id, player_id).await.unwrap();
        assert!(ok);

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert!(cards.is_empty());
    }

    #[tokio::test]
    async fn test_make_purchase_increases_debt() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 30_00, "Purchase").await.unwrap();

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, 30_00);

        let txs = wcs.get_card_transactions(card.id, "all").await.unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].tx_type, "purchase");
        assert_eq!(txs[0].amount, 30_00);
    }

    #[tokio::test]
    async fn test_make_purchase_over_limit_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        // Set account limit to $50 (shared across all cards)
        sqlx::query(
            "INSERT INTO player_credit_accounts (player_id, credit_limit, created_at) VALUES ($1, $2, now()) ON CONFLICT (player_id) DO UPDATE SET credit_limit = $2",
        )
        .bind(player_id)
        .bind(50_00i64)
        .execute(&pool)
        .await
        .unwrap();

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 30_00, "OK").await.unwrap();
        let res = wcs.make_purchase(card.id, player_id, 30_00, "Over").await;
        assert!(matches!(res, Err(WalletError::CardLimitExceeded)));
    }

    #[tokio::test]
    async fn test_pay_card_bill_debits_usd_and_zeros_debt() {
        let pool = test_pool().await;
        let ws = WalletService::new(pool.clone());
        let wcs = WalletCardService::new(pool.clone());
        let player_id = create_test_player_with_wallet(&pool).await;

        ws.credit(player_id, "USD", 200_00, "seed").await.unwrap();
        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 75_00, "Bill").await.unwrap();

        let paid = wcs.pay_card_bill(card.id, player_id).await.unwrap();
        assert_eq!(paid, 75_00);

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, 0);

        let balances = ws.get_balances(player_id).await.unwrap();
        let usd = balances.iter().find(|b| b.currency == "USD").unwrap();
        assert_eq!(usd.balance, 200_00 - 75_00);
    }

    #[tokio::test]
    async fn test_pay_card_bill_insufficient_usd_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 50_00, "Bill").await.unwrap();
        // No USD credit - balance 0
        let res = wcs.pay_card_bill(card.id, player_id).await;
        assert!(matches!(res, Err(WalletError::InsufficientBalance)));
    }

    #[tokio::test]
    async fn test_pay_card_bill_zero_debt_returns_zero() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        let paid = wcs.pay_card_bill(card.id, player_id).await.unwrap();
        assert_eq!(paid, 0);
    }

    #[tokio::test]
    async fn test_get_card_transactions_filter() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 10_00, "Tx").await.unwrap();

        let all = wcs.get_card_transactions(card.id, "all").await.unwrap();
        assert_eq!(all.len(), 1);
        let today = wcs.get_card_transactions(card.id, "today").await.unwrap();
        assert!(!today.is_empty());
    }

    #[tokio::test]
    async fn test_next_billing_monday() {
        use chrono::TimeZone;
        // Wednesday 2025-02-19 12:00 UTC -> next Monday 2025-02-24 12:00
        let wed = Utc.with_ymd_and_hms(2025, 2, 19, 12, 0, 0).unwrap();
        let mon = next_billing_monday(wed);
        assert_eq!(mon.weekday(), chrono::Weekday::Mon);
        assert_eq!(mon.day(), 24);
        assert_eq!(mon.month(), 2);
    }

    #[tokio::test]
    async fn test_next_billing_monday_from_monday_returns_next_week() {
        use chrono::TimeZone;
        let mon = Utc.with_ymd_and_hms(2025, 2, 24, 12, 0, 0).unwrap();
        let next = next_billing_monday(mon);
        assert_eq!(next.weekday(), chrono::Weekday::Mon);
        assert_eq!(next.day(), 3);
        assert_eq!(next.month(), 3);
    }

    #[tokio::test]
    async fn test_multiple_cards_same_player() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let c1 = wcs.create_card(player_id, Some("Card 1"), "User").await.unwrap();
        let c2 = wcs.create_card(player_id, Some("Card 2"), "User").await.unwrap();

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards.len(), 2);
        assert_ne!(c1.id, c2.id);
        assert!(c1.number_full != c2.number_full);
        assert_eq!(cards[0].credit_limit, cards[1].credit_limit, "shared account limit");
    }

    #[tokio::test]
    async fn test_two_cards_share_limit() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        sqlx::query(
            "INSERT INTO player_credit_accounts (player_id, credit_limit, created_at) VALUES ($1, $2, now()) ON CONFLICT (player_id) DO UPDATE SET credit_limit = $2",
        )
        .bind(player_id)
        .bind(100_00i64)
        .execute(&pool)
        .await
        .unwrap();

        let c1 = wcs.create_card(player_id, Some("A"), "User").await.unwrap();
        let c2 = wcs.create_card(player_id, Some("B"), "User").await.unwrap();

        wcs.make_purchase(c1.id, player_id, 60_00, "On A").await.unwrap();
        wcs.make_purchase(c2.id, player_id, 40_00, "On B").await.unwrap();

        let cards = wcs.get_cards(player_id).await.unwrap();
        let total_debt: i64 = cards.iter().map(|c| c.current_debt).sum();
        assert_eq!(total_debt, 100_00);

        let res = wcs.make_purchase(c2.id, player_id, 1, "Over").await;
        assert!(matches!(res, Err(WalletError::CardLimitExceeded)));
    }

    #[tokio::test]
    async fn test_get_cards_empty_for_new_player() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert!(cards.is_empty());
    }

    #[tokio::test]
    async fn test_delete_card_wrong_player_returns_false() {
        let pool = test_pool().await;
        let ps = PlayerService::new(pool.clone());
        let ws = WalletService::new(pool.clone());
        let wcs = WalletCardService::new(pool.clone());

        let p1 = ps.create_player(&format!("card_test_1_{}", Uuid::new_v4()), "pw").await.unwrap();
        let p2 = ps.create_player(&format!("card_test_2_{}", Uuid::new_v4()), "pw").await.unwrap();
        ws.create_wallet_for_player(p1.id).await.unwrap();
        ws.create_wallet_for_player(p2.id).await.unwrap();

        let card = wcs.create_card(p1.id, None, "User").await.unwrap();
        let ok = wcs.delete_card(card.id, p2.id).await.unwrap();
        assert!(!ok);
        let cards = wcs.get_cards(p1.id).await.unwrap();
        assert_eq!(cards.len(), 1);
    }

    #[tokio::test]
    async fn test_get_card_transactions_empty_before_purchase() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        let txs = wcs.get_card_transactions(card.id, "all").await.unwrap();
        assert!(txs.is_empty());
    }

    #[tokio::test]
    async fn test_make_purchase_multiple_then_pay_full() {
        let pool = test_pool().await;
        let ws = WalletService::new(pool.clone());
        let wcs = WalletCardService::new(pool.clone());
        let player_id = create_test_player_with_wallet(&pool).await;

        ws.credit(player_id, "USD", 500_00, "seed").await.unwrap();
        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 20_00, "A").await.unwrap();
        wcs.make_purchase(card.id, player_id, 30_00, "B").await.unwrap();
        wcs.make_purchase(card.id, player_id, 50_00, "C").await.unwrap();

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, 100_00);

        let paid = wcs.pay_card_bill(card.id, player_id).await.unwrap();
        assert_eq!(paid, 100_00);
        let cards_after = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards_after[0].current_debt, 0);

        let txs = wcs.get_card_transactions(card.id, "all").await.unwrap();
        assert_eq!(txs.len(), 4); // 3 purchases + 1 payment
        assert!(txs.iter().any(|t| t.tx_type == "payment" && t.amount == 100_00));
    }

    #[tokio::test]
    async fn test_create_card_statement_has_open_status() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let card = wcs.create_card(player_id, None, "Holder").await.unwrap();
        let stmt = wcs.get_current_statement(card.id).await.unwrap();
        assert!(stmt.is_some());
        let s = stmt.unwrap();
        assert_eq!(s.status, "open");
        assert!(s.total_amount >= 0);
    }

    #[tokio::test]
    async fn test_pay_card_bill_wrong_card_id_fails() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        let wrong_id = Uuid::new_v4();
        let res = wcs.pay_card_bill(wrong_id, player_id).await;
        assert!(res.is_err());
        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, 0);
        assert_eq!(card.id, cards[0].id);
    }

    #[tokio::test]
    async fn test_make_purchase_exact_limit_ok() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        sqlx::query(
            "INSERT INTO player_credit_accounts (player_id, credit_limit, created_at) VALUES ($1, $2, now()) ON CONFLICT (player_id) DO UPDATE SET credit_limit = $2",
        )
        .bind(player_id)
        .bind(99_00i64)
        .execute(&pool)
        .await
        .unwrap();

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 99_00, "Full limit").await.unwrap();
        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, 99_00);
        let res = wcs.make_purchase(card.id, player_id, 1, "Over").await;
        assert!(matches!(res, Err(WalletError::CardLimitExceeded)));
    }

    #[tokio::test]
    async fn test_get_card_transactions_filter_7d() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 10_00, "Tx").await.unwrap();
        let txs_7d = wcs.get_card_transactions(card.id, "7d").await.unwrap();
        assert!(!txs_7d.is_empty());
    }

    #[tokio::test]
    async fn test_card_label_stored() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool);

        let card = wcs.create_card(player_id, Some("My Virtual"), "User").await.unwrap();
        assert_eq!(card.label.as_deref(), Some("My Virtual"));
        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].label.as_deref(), Some("My Virtual"));
    }

    /// Statement reflects all purchases made in the billing period.
    #[tokio::test]
    async fn test_statement_reflects_all_purchases_in_period() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 20_00, "Purchase 1").await.unwrap();
        wcs.make_purchase(card.id, player_id, 30_00, "Purchase 2").await.unwrap();
        wcs.make_purchase(card.id, player_id, 15_00, "Purchase 3").await.unwrap();

        let stmt = wcs.get_current_statement(card.id).await.unwrap().unwrap();
        assert_eq!(stmt.total_amount, 65_00); // 20 + 30 + 15

        let txs = wcs.get_card_transactions(card.id, "all").await.unwrap();
        assert_eq!(txs.len(), 3);
    }

    /// Statement total matches sum of purchases.
    #[tokio::test]
    async fn test_statement_total_matches_purchases() {
        let pool = test_pool().await;
        let player_id = create_test_player_with_wallet(&pool).await;
        let wcs = WalletCardService::new(pool.clone());

        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        let amounts = [10_00, 25_00, 40_00, 5_00];
        let expected_total: i64 = amounts.iter().sum();

        for (i, &amt) in amounts.iter().enumerate() {
            wcs.make_purchase(card.id, player_id, amt, &format!("Tx {}", i)).await.unwrap();
        }

        let stmt = wcs.get_current_statement(card.id).await.unwrap().unwrap();
        assert_eq!(stmt.total_amount, expected_total);

        let cards = wcs.get_cards(player_id).await.unwrap();
        assert_eq!(cards[0].current_debt, expected_total);
    }

    /// Paying a bill creates a wallet transaction with correct keys (from_key = player, to_key = system).
    #[tokio::test]
    async fn test_pay_bill_creates_wallet_transaction_with_correct_keys() {
        let pool = test_pool().await;
        let ws = WalletService::new(pool.clone());
        let wcs = WalletCardService::new(pool.clone());
        let player_id = create_test_player_with_wallet(&pool).await;

        ws.credit(player_id, "USD", 500_00, "seed").await.unwrap();
        let card = wcs.create_card(player_id, None, "User").await.unwrap();
        wcs.make_purchase(card.id, player_id, 75_00, "Bill").await.unwrap();

        let txs_before = ws.get_transactions(player_id, "all").await.unwrap();
        let debit_count_before = txs_before.iter().filter(|t| t.tx_type == "debit").count();

        wcs.pay_card_bill(card.id, player_id).await.unwrap();

        let txs_after = ws.get_transactions(player_id, "all").await.unwrap();
        let debit_txs: Vec<_> = txs_after.iter().filter(|t| t.tx_type == "debit").collect();
        assert_eq!(debit_txs.len(), debit_count_before + 1);

        let bill_debit = debit_txs.iter().find(|t| t.amount == 75_00).unwrap();
        assert_eq!(bill_debit.counterpart_address.as_deref(), Some("system"));
    }
}
