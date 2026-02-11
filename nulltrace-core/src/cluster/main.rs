mod process;
mod vm;
mod os;
mod net;
mod db;
mod lua_api;
mod vm_manager;

use db::fs_service::FsService;
use db::user_service::UserService;
use db::vm_service::VmService;
use lua_api::context::VmContext;
use net::ip::{Ipv4Addr, Subnet};
use os::create_lua_state;
use std::sync::Arc;
use vm::VirtualMachine;
use vm_manager::VmManager;

const DATABASE_URL: &str = "postgres://nulltrace:nulltrace@localhost:5432/nulltrace";
const REDIS_URL: &str = "redis://127.0.0.1:6379";

#[tokio::main]
async fn main() {
    // ── Database ──
    let pool = db::connect(DATABASE_URL)
        .await
        .expect("Failed to connect to database");
    println!("[cluster] Connected to PostgreSQL");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    println!("[cluster] Migrations applied");

    let vm_service = Arc::new(VmService::new(pool.clone()));
    let fs_service = Arc::new(FsService::new(pool.clone()));
    let user_service = Arc::new(UserService::new(pool.clone()));

    // ── Lua state ──
    let lua = create_lua_state();
    lua.set_app_data(VmContext::new(pool.clone()));
    lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).expect("Failed to register Lua APIs");
    println!("[cluster] Lua APIs registered (fs, net, os)");

    // ── VM Manager ──
    let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);
    let mut manager = VmManager::new(vm_service.clone(), fs_service.clone(), user_service.clone(), subnet);

    // ── Redis (optional — cluster works without it) ──
    match manager.net_manager.connect_redis(REDIS_URL) {
        Ok(()) => println!("[cluster] Connected to Redis"),
        Err(e) => println!("[cluster] Redis not available ({}), running without cross-pod support", e),
    }

    // ── Restore VMs from DB ──
    let records = manager
        .restore_vms()
        .await
        .expect("Failed to restore VMs");
    println!("[cluster] Restored {} VMs from database", records.len());

    let mut vms: Vec<VirtualMachine> = Vec::new();

    for record in &records {
        let mut vm = VirtualMachine::with_id(&lua, record.id);

        // Re-attach NIC if VM had an IP
        if let (Some(ip_str), Some(subnet_str)) = (&record.ip, &record.subnet) {
            if let (Some(ip), Some(subnet)) = (Ipv4Addr::parse(ip_str), Subnet::parse(subnet_str))
            {
                let nic = net::nic::NIC::new(ip, subnet);
                vm.attach_nic(nic);
            }
        }

        println!(
            "[cluster]   {} ({}) - {}",
            record.hostname,
            record.ip.as_deref().unwrap_or("no ip"),
            record.status,
        );

        vms.push(vm);
    }

    println!(
        "[cluster] Ready. {} VMs active. Starting game loop...",
        vms.len()
    );

    // ── Game loop ──
    manager.run_loop(&lua, &mut vms).await;
}
