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

    /// Returns all active cards for a player.
    pub async fn get_cards(&self, player_id: Uuid) -> Result<Vec<WalletCard>, WalletError> {
        let rows = sqlx::query_as::<_, WalletCard>(
            r#"
            SELECT id, player_id, label, number_full, last4, expiry_month, expiry_year,
                   cvv, holder_name, credit_limit, current_debt, is_virtual, is_active,
                   billing_day_of_week, created_at
            FROM wallet_cards
            WHERE player_id = $1 AND is_active = TRUE
            ORDER BY created_at ASC
            "#,
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Creates a new virtual card and opens its first statement.
    pub async fn create_card(
        &self,
        player_id: Uuid,
        label: Option<&str>,
        holder_name: &str,
        credit_limit: i64,
    ) -> Result<WalletCard, WalletError> {
        let number = generate_card_number();
        let last4 = number[12..].to_string();
        let cvv = generate_cvv();
        let now = Utc::now();
        let expiry_year = now.year() + 3;
        let expiry_month = now.month() as i32;

        let card = sqlx::query_as::<_, WalletCard>(
            r#"
            INSERT INTO wallet_cards
                (id, player_id, label, number_full, last4, expiry_month, expiry_year,
                 cvv, holder_name, credit_limit, current_debt, is_virtual, is_active, billing_day_of_week)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, TRUE, TRUE, 1)
            RETURNING id, player_id, label, number_full, last4, expiry_month, expiry_year,
                      cvv, holder_name, credit_limit, current_debt, is_virtual, is_active,
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
        .bind(credit_limit)
        .fetch_one(&self.pool)
        .await?;

        // Open first billing statement
        self.get_or_create_open_statement(card.id).await?;

        Ok(card)
    }

    /// Soft-deletes a card (is_active = false). Returns true if the card was found.
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
        Ok(result.rows_affected() > 0)
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

    /// Records a purchase on the card. Fails with CardLimitExceeded if it would exceed the limit.
    /// Also updates the current open statement's total.
    pub async fn make_purchase(
        &self,
        card_id: Uuid,
        player_id: Uuid,
        amount: i64,
        description: &str,
    ) -> Result<(), WalletError> {
        // Increment debt only if still within limit
        let updated = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE wallet_cards
            SET current_debt = current_debt + $1
            WHERE id = $2 AND player_id = $3 AND is_active = TRUE
              AND (current_debt + $1) <= credit_limit
            RETURNING current_debt
            "#,
        )
        .bind(amount)
        .bind(card_id)
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await?;

        if updated.is_none() {
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
        .execute(&self.pool)
        .await?;

        // Accumulate into current open statement
        let stmt = self.get_or_create_open_statement(card_id).await?;
        sqlx::query(
            "UPDATE wallet_card_statements SET total_amount = total_amount + $1 WHERE id = $2",
        )
        .bind(amount)
        .bind(stmt.id)
        .execute(&self.pool)
        .await?;

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

        // Lock and fetch the card's current debt
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT current_debt FROM wallet_cards
            WHERE id = $1 AND player_id = $2 AND is_active = TRUE
            FOR UPDATE
            "#,
        )
        .bind(card_id)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;

        let debt = match row {
            Some((d,)) => d,
            None => {
                tx.rollback().await?;
                return Err(WalletError::InvalidCurrency); // card not found
            }
        };

        if debt == 0 {
            tx.rollback().await?;
            return Ok(0);
        }

        // Debit USD from wallet (only if sufficient balance)
        let deducted = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE wallet_accounts
            SET balance = balance - $1, updated_at = now()
            WHERE player_id = $2 AND currency = 'USD' AND balance >= $1
            RETURNING balance
            "#,
        )
        .bind(debt)
        .bind(player_id)
        .fetch_optional(&mut *tx)
        .await?;

        if deducted.is_none() {
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

        // Log wallet debit
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (id, player_id, tx_type, currency, amount, fee, description)
            VALUES ($1, $2, 'debit', 'USD', $3, 0, 'Credit card bill payment')
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(player_id)
        .bind(debt)
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
