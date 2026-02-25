#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;
use uuid::Uuid;

use super::fkebank_account_service::FkebankAccountService;
use super::wallet_card_service::WalletCardService;
use super::wallet_common::WalletError;

#[derive(Debug, Clone, FromRow)]
pub struct CardInvoice {
    pub id: Uuid,
    pub destination_key: String,
    pub amount_cents: i64,
    pub fee_percent: i32,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct CardInvoiceService {
    pool: PgPool,
    fkebank: Arc<FkebankAccountService>,
    wallet_card: Arc<WalletCardService>,
}

impl CardInvoiceService {
    pub fn new(
        pool: PgPool,
        fkebank: Arc<FkebankAccountService>,
        wallet_card: Arc<WalletCardService>,
    ) -> Self {
        Self {
            pool,
            fkebank,
            wallet_card,
        }
    }

    /// Creates an invoice. Validates that destination_key exists in Fkebank.
    pub async fn create_invoice(
        &self,
        destination_key: &str,
        amount_cents: i64,
    ) -> Result<CardInvoice, WalletError> {
        if amount_cents <= 0 {
            return Err(WalletError::InvalidCurrency);
        }
        let exists = self.fkebank.get_by_key(destination_key).await?;
        if exists.is_none() {
            return Err(WalletError::RecipientNotFound);
        }
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, CardInvoice>(
            r#"
            INSERT INTO card_invoices (id, destination_key, amount_cents, fee_percent, status, created_at)
            VALUES ($1, $2, $3, 5, 'pending', now())
            RETURNING id, destination_key, amount_cents, fee_percent, status, paid_at, created_at
            "#,
        )
        .bind(id)
        .bind(destination_key)
        .bind(amount_cents)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Pays an invoice with a card. Validates card, checks limit, credits merchant (95%), 5% is lost.
    pub async fn pay_invoice(
        &self,
        invoice_id: Uuid,
        card_number: &str,
        cvv: &str,
        expiry_month: i32,
        expiry_year: i32,
        holder_name: &str,
    ) -> Result<(), WalletError> {
        let invoice = sqlx::query_as::<_, CardInvoice>(
            "SELECT id, destination_key, amount_cents, fee_percent, status, paid_at, created_at FROM card_invoices WHERE id = $1",
        )
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await?;

        let invoice = invoice.ok_or(WalletError::InvoiceNotFound)?;
        if invoice.status != "pending" {
            return Err(WalletError::InvoiceAlreadyPaid);
        }

        let (card, player_id) = self
            .wallet_card
            .find_card_by_number(card_number, cvv, expiry_month, expiry_year, holder_name)
            .await?
            .ok_or(WalletError::CardNotFound)?;

        self.wallet_card
            .make_purchase(card.id, player_id, invoice.amount_cents, "Invoice payment")
            .await?;

        let merchant_amount = (invoice.amount_cents as i64 * (100 - invoice.fee_percent) as i64) / 100;
        self.fkebank
            .credit(&invoice.destination_key, merchant_amount, "Card invoice payment")
            .await?;

        sqlx::query(
            "UPDATE card_invoices SET status = 'paid', paid_at = now() WHERE id = $1",
        )
        .bind(invoice_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Returns total amount (in cents) collected for a destination key (sum of amount_cents for paid invoices).
    pub async fn get_total_collected(&self, destination_key: &str) -> Result<i64, WalletError> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            SELECT (COALESCE(SUM(amount_cents), 0))::bigint FROM card_invoices
            WHERE destination_key = $1 AND status = 'paid'
            "#,
        )
        .bind(destination_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::super::fkebank_account_service::FkebankAccountService;
    use super::super::player_service::PlayerService;
    use super::super::test_pool;
    use super::super::wallet_card_service::WalletCardService;
    use super::super::wallet_service::WalletService;
    use super::*;
    use std::sync::Arc;
    use uuid::Uuid;

    fn unique_username() -> String {
        format!("card_inv_test_{}", Uuid::new_v4())
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
    async fn test_create_invoice_valid_destination() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("test-dest-{}", Uuid::new_v4());
        let account = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&account.key, 10_000)
            .await
            .unwrap();
        assert_eq!(invoice.destination_key, account.key);
        assert_eq!(invoice.amount_cents, 10_000);
        assert_eq!(invoice.fee_percent, 5);
        assert_eq!(invoice.status, "pending");
    }

    #[tokio::test]
    async fn test_create_invoice_invalid_destination_fails() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let res = svc.create_invoice("nonexistent-key-12345", 10_000).await;
        assert!(matches!(res, Err(WalletError::RecipientNotFound)));
    }

    #[tokio::test]
    async fn test_pay_invoice_success() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let player_id = create_test_player_with_wallet(&pool).await;
        let card = wcs
            .create_card(player_id, None, "Test User")
            .await
            .unwrap();

        svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            card.expiry_month,
            card.expiry_year,
            &card.holder_name,
        )
        .await
        .unwrap();

        let balance = fkebank.get_balance_by_key(&merchant.key).await.unwrap();
        assert_eq!(balance, 9_500); // 95% of 10000
    }

    #[tokio::test]
    async fn test_pay_invoice_over_limit_fails() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let player_id = create_test_player_with_wallet(&pool).await;
        sqlx::query(
            "INSERT INTO player_credit_accounts (player_id, credit_limit, created_at) VALUES ($1, $2, now()) ON CONFLICT (player_id) DO UPDATE SET credit_limit = $2",
        )
        .bind(player_id)
        .bind(5_000i64)
        .execute(&pool)
        .await
        .unwrap();
        let card = wcs
            .create_card(player_id, None, "User")
            .await
            .unwrap();

        let res = svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            card.expiry_month,
            card.expiry_year,
            &card.holder_name,
        )
        .await;
        assert!(matches!(res, Err(WalletError::CardLimitExceeded)));
    }

    #[tokio::test]
    async fn test_pay_invoice_invalid_card_fails() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let res = svc.pay_invoice(
            invoice.id,
            "4111111111111111",
            "123",
            12,
            2030,
            "Unknown",
        )
        .await;
        assert!(matches!(res, Err(WalletError::CardNotFound)));
    }

    #[tokio::test]
    async fn test_pay_invoice_already_paid_fails() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let player_id = create_test_player_with_wallet(&pool).await;
        let card = wcs
            .create_card(player_id, None, "User")
            .await
            .unwrap();

        svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            card.expiry_month,
            card.expiry_year,
            &card.holder_name,
        )
        .await
        .unwrap();

        let res = svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            card.expiry_month,
            card.expiry_year,
            &card.holder_name,
        )
        .await;
        assert!(matches!(res, Err(WalletError::InvoiceAlreadyPaid)));
    }

    #[tokio::test]
    async fn test_pay_invoice_expired_card_fails() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let player_id = create_test_player_with_wallet(&pool).await;
        let card = wcs
            .create_card(player_id, None, "User")
            .await
            .unwrap();

        sqlx::query(
            "UPDATE wallet_cards SET expiry_year = 2020, expiry_month = 1 WHERE id = $1",
        )
        .bind(card.id)
        .execute(&pool)
        .await
        .unwrap();

        let res = svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            1,
            2020,
            &card.holder_name,
        )
        .await;
        assert!(matches!(res, Err(WalletError::CardNotFound)));
    }

    #[tokio::test]
    async fn test_get_total_collected() {
        let pool = test_pool().await;
        let fkebank = Arc::new(FkebankAccountService::new(pool.clone()));
        let wcs = Arc::new(WalletCardService::new(pool.clone()));
        let svc = CardInvoiceService::new(pool.clone(), fkebank.clone(), wcs.clone());

        let account_id = format!("merchant-{}", Uuid::new_v4());
        let merchant = fkebank
            .create_account_for_account_id(&account_id, None, None)
            .await
            .unwrap();

        let total_before = svc.get_total_collected(&merchant.key).await.unwrap();
        assert_eq!(total_before, 0);

        let invoice = svc
            .create_invoice(&merchant.key, 10_000)
            .await
            .unwrap();

        let player_id = create_test_player_with_wallet(&pool).await;
        let card = wcs
            .create_card(player_id, None, "User")
            .await
            .unwrap();

        svc.pay_invoice(
            invoice.id,
            &card.number_full,
            &card.cvv,
            card.expiry_month,
            card.expiry_year,
            &card.holder_name,
        )
        .await
        .unwrap();

        let total_after = svc.get_total_collected(&merchant.key).await.unwrap();
        assert_eq!(total_after, 10_000);
    }
}
