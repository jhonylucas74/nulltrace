pub mod admin_service;
pub mod vm_service;
pub mod fs_service;
pub mod user_service;
pub mod player_service;
pub mod faction_service;
pub mod faction_invite_service;
pub mod faction_member_service;
pub mod hackerboard_dm_service;
pub mod hackerboard_faction_chat_service;
pub mod shortcuts_service;
pub mod email_service;
pub mod email_account_service;
pub mod wallet_common;
pub mod fkebank_account_service;
pub mod crypto_wallet_service;
pub mod wallet_service;
pub mod wallet_card_service;
pub mod card_invoice_service;
pub mod codelab_service;
pub mod feed_service;
pub mod player_block_service;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[cfg(test)]
const DEFAULT_DATABASE_URL: &str = "postgres://nulltrace:nulltrace@localhost:5432/nulltrace";

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(100)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(include_str!("../../../migrations/001_create_vms.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/002_create_fs_nodes.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/003_create_fs_contents.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/004_add_dns_name.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/005_create_vm_users.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/006_create_players.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/007_content_addressable_blobs.sql"))
        .execute(pool)
        .await?;
    fs_service::FsService::migrate_fs_contents_to_blob_store(pool).await?;
    sqlx::raw_sql(include_str!("../../../migrations/008_drop_fs_contents_data.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/009_add_player_points.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/010_create_factions.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/011_add_player_preferred_theme.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/012_create_player_shortcuts.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/013_create_emails.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/014_create_email_accounts.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/015_add_emails_cc_address.sql"))
        .execute(pool)
        .await?;
    fs_service::FsService::cleanup_orphan_blobs(pool).await?;
    sqlx::raw_sql(include_str!("../../../migrations/016_create_fkebank_accounts.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/017_create_fkebank_tokens.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/018_create_crypto_wallets.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/019_create_wallet_transactions_keybased.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/020_create_wallet_cards.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/021_create_wallet_card_transactions.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/022_create_wallet_card_statements.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/023_add_npc_owner_type.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/024_create_card_invoices.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/025_fix_card_limit_zero.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/026_create_player_credit_accounts.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/027_shared_credit_limit.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/028_create_codelab_progress.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/029_add_vm_internet_plan.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/030_create_admins.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/031_create_feed_posts.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/032_add_player_hackerboard_language_prefs.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/033_create_faction_invites.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/034_hackerboard_direct_messages.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/035_hackerboard_faction_chat.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/036_player_blocks.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/037_faction_invite_permissions.sql"))
        .execute(pool)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/038_faction_member_bans.sql"))
        .execute(pool)
        .await?;
    Ok(())
}

/// Creates a test pool and runs migrations. Used by integration tests.
/// Run with `cargo test --bin cluster -- --test-threads=1` to avoid migration deadlocks and ensure DB isolation.
#[cfg(test)]
pub async fn test_pool() -> PgPool {
    let pool = connect(DEFAULT_DATABASE_URL)
        .await
        .expect("Failed to connect to test database. Is PostgreSQL running?");
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}
