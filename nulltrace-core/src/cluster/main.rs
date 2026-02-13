mod auth;
mod bench_scripts;
mod bin_programs;
mod path_util;
mod process;
mod process_parser;
mod vm;
mod os;
mod net;
mod db;
mod grpc;
mod lua_api;
mod terminal_hub;
mod vm_manager;
mod vm_worker;

use dashmap::DashMap;
use db::faction_service::FactionService;
use db::fs_service::FsService;
use db::player_service::PlayerService;
use db::user_service::UserService;
use db::vm_service::VmService;
use grpc::game::game_service_server::GameServiceServer;
use grpc::ClusterGameService;
use lua_api::context::VmContext;
use terminal_hub::new_hub;
use net::ip::{Ipv4Addr, Subnet};
use os::create_lua_state;
use std::sync::Arc;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use vm::VirtualMachine;
use vm_manager::{ProcessSnapshot, VmManager};
use db::vm_service::VmConfig;

const DATABASE_URL: &str = "postgres://nulltrace:nulltrace@localhost:5432/nulltrace";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const GRPC_ADDR: &str = "127.0.0.1:50051";

#[tokio::main]
async fn main() {
    // Detectar modo (STRESS_TEST env var ativa modo stress)
    let stress_mode = std::env::var("STRESS_TEST").is_ok();

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
    let player_service = Arc::new(PlayerService::new(pool.clone()));
    let faction_service = Arc::new(FactionService::new(pool.clone()));

    // ── Seed default player (Haru) if not present ──
    player_service
        .seed_haru()
        .await
        .expect("Failed to seed default player");
    println!("[cluster] Default player ready (Haru)");

    // ── Lua state ──
    let lua = create_lua_state();
    lua.set_app_data(VmContext::new(pool.clone()));
    lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).expect("Failed to register Lua APIs");
    println!("[cluster] Lua APIs registered (fs, net, os)");

    // ── VM Manager ──
    let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 22);
    let mut manager = VmManager::new(
        vm_service.clone(),
        fs_service.clone(),
        user_service.clone(),
        player_service.clone(),
        subnet,
    );

    // ── Redis (optional — cluster works without it) ──
    match manager.net_manager.connect_redis(REDIS_URL) {
        Ok(()) => println!("[cluster] Connected to Redis"),
        Err(e) => println!("[cluster] Redis not available ({}), running without cross-pod support", e),
    }

    // ── Criar ou carregar VMs dependendo do modo ──
    let mut vms = if stress_mode {
        create_stress_vms(&lua)
    } else {
        load_game_vms(&mut manager, &lua, &player_service, &vm_service).await
    };

    let terminal_hub = new_hub();

    let process_snapshot_store: Arc<DashMap<uuid::Uuid, Vec<ProcessSnapshot>>> =
        Arc::new(DashMap::new());

    // ── gRPC server (runs in background task) ──
    let grpc_addr = GRPC_ADDR.parse().expect("Invalid gRPC address");
    let game_svc = ClusterGameService::new(
        player_service.clone(),
        vm_service.clone(),
        fs_service.clone(),
        user_service.clone(),
        faction_service.clone(),
        terminal_hub.clone(),
        process_snapshot_store.clone(),
    );
    let game_server = GameServiceServer::new(game_svc);
    tokio::spawn(async move {
        Server::builder()
            .accept_http1(true)
            .layer(GrpcWebLayer::new())
            .add_service(game_server)
            .serve(grpc_addr)
            .await
            .expect("gRPC server failed");
    });
    println!("[cluster] gRPC server listening on {}", GRPC_ADDR);

    // ── Game loop (main task) ──
    manager
        .run_loop(&lua, &mut vms, terminal_hub, process_snapshot_store, &pool, stress_mode)
        .await;
}

/// Criar 5k VMs in-memory para stress test
fn create_stress_vms(lua: &mlua::Lua) -> Vec<VirtualMachine> {
    println!("[cluster] STRESS TEST MODE: Creating 5,000 VMs (in-memory)...");
    let mut vms = Vec::new();
    let start_creation = std::time::Instant::now();

    for i in 0..5_000 {
        let mut vm = VirtualMachine::new(lua);

        // Spawn infinite loop process in each VM
        vm.os.spawn_process(
            r#"
local count = 0
while true do
    count = count + 1
    print("VM tick: " .. count)
end
            "#,
            vec![],
            0,
            "root",
        );

        vms.push(vm);

        if (i + 1) % 500 == 0 {
            println!("[cluster]   Created {}/5,000 VMs...", i + 1);
        }
    }

    let creation_time = start_creation.elapsed();
    println!(
        "[cluster] ✓ Created {} VMs in {:.2}s ({:.0} VMs/sec)",
        vms.len(),
        creation_time.as_secs_f64(),
        vms.len() as f64 / creation_time.as_secs_f64()
    );
    println!("[cluster] Starting 20-second stress test...\n");

    vms
}

/// Carregar VMs do banco de dados para modo jogo
async fn load_game_vms<'a>(
    manager: &mut VmManager,
    lua: &'a mlua::Lua,
    player_service: &Arc<PlayerService>,
    vm_service: &Arc<VmService>,
) -> Vec<VirtualMachine<'a>> {
    println!("[cluster] GAME MODE: Loading VMs from database...");

    let mut vms = Vec::new();

    // 1. Ensure Haru has a VM (create if doesn't exist)
    let haru = player_service
        .get_by_username("Haru")
        .await
        .expect("Failed to get Haru")
        .expect("Haru not found");

    let haru_vm = vm_service
        .get_vm_by_owner_id(haru.id)
        .await
        .expect("Failed to check Haru's VM");

    if haru_vm.is_none() {
        println!("[cluster] Creating VM for Haru...");
        let config = VmConfig {
            hostname: "haru-desktop".to_string(),
            dns_name: Some("haru.local".to_string()),
            cpu_cores: 2,
            memory_mb: 2048,
            disk_mb: 20480,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: Some(haru.id),
        };
        manager.create_vm(config).await.expect("Failed to create Haru's VM");
        println!("[cluster] ✓ Haru's VM created");

        // Clear active_vms to avoid duplication when restore_vms() runs
        manager.clear_active_vms();
    }

    // 2. Restore all running/crashed player VMs from database
    match manager.restore_vms().await {
        Ok(records) => {
            println!("[cluster] Restored {} VM(s) from database", records.len());
        }
        Err(e) => {
            println!("[cluster] Warning: Failed to restore VMs: {}", e);
            return vms;
        }
    }

    // 2. Get all active VM IDs (already registered in manager by restore_vms)
    let active_vm_ids: Vec<uuid::Uuid> = manager
        .get_active_vms()
        .iter()
        .map(|a| a.id)
        .collect();

    // 3. Create VirtualMachine instance for each restored VM
    for vm_id in active_vm_ids {
        let mut vm = VirtualMachine::with_id(lua, vm_id);

        // Attach NIC if the VM has an IP assigned
        if let Some(active_vm) = manager.get_active_vm(vm_id) {
            if let Some(ip) = active_vm.ip {
                let subnet = manager.subnet.clone();
                let nic = net::nic::NIC::new(ip, subnet);
                vm.attach_nic(nic);
                println!("[cluster]   VM {} ({}) - IP: {}", active_vm.hostname, vm_id, ip);
            } else {
                println!("[cluster]   VM {} ({}) - No IP assigned", active_vm.hostname, vm_id);
            }
        }

        vms.push(vm);
    }

    println!("[cluster] ✓ Loaded {} VMs from database", vms.len());
    vms
}
