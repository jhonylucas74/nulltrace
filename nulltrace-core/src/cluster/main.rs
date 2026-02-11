mod process;
mod vm;
mod os;
mod net;
mod db;

use std::sync::Arc;

const DATABASE_URL: &str = "postgres://nulltrace:nulltrace@localhost:5432/nulltrace";

#[tokio::main]
async fn main() {
    // Connect to PostgreSQL
    let pool = db::connect(DATABASE_URL).await.expect("Failed to connect to database");
    println!("[cluster] Connected to PostgreSQL");

    // Run migrations
    db::run_migrations(&pool).await.expect("Failed to run migrations");
    println!("[cluster] Migrations applied");

    let vm_service = Arc::new(db::vm_service::VmService::new(pool.clone()));
    let _fs_service = Arc::new(db::fs_service::FsService::new(pool.clone()));

    // Restore VMs that were running before crash/shutdown
    let vms_to_restore = vm_service.restore_running_vms().await.expect("Failed to restore VMs");
    println!("[cluster] Restoring {} VMs", vms_to_restore.len());

    for record in &vms_to_restore {
        println!(
            "[cluster] Restored VM {} (hostname: {}, ip: {})",
            record.id,
            record.hostname,
            record.ip.as_deref().unwrap_or("none"),
        );
    }

    println!("[cluster] Ready. {} VMs active.", vms_to_restore.len());
}
