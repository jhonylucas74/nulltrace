pub mod vm_service;
pub mod fs_service;
pub mod user_service;
pub mod player_service;
pub mod faction_service;
pub mod shortcuts_service;

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
