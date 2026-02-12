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
use vm_manager::VmManager;

const DATABASE_URL: &str = "postgres://nulltrace:nulltrace@localhost:5432/nulltrace";
const REDIS_URL: &str = "redis://127.0.0.1:6379";
const GRPC_ADDR: &str = "127.0.0.1:50051";

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
    let subnet = Subnet::new(Ipv4Addr::new(10, 0, 1, 0), 24);
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

    // ── Seed Haru's VM (single test VM) ──
    let haru = player_service
        .get_by_username(db::player_service::SEED_USERNAME)
        .await
        .expect("Failed to get Haru player")
        .expect("Haru player not found");

    // Check if Haru already has a VM
    let existing_vms = vm_service
        .list_all()
        .await
        .expect("Failed to list VMs");

    let haru_vm_exists = existing_vms.iter().any(|v| v.owner_id == Some(haru.id));

    let mut vms: Vec<VirtualMachine> = Vec::new();

    if !haru_vm_exists {
        println!("[cluster] Creating Haru's VM...");
        let config = db::vm_service::VmConfig {
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

        let (record, nic) = manager
            .create_vm(config)
            .await
            .expect("Failed to create Haru's VM");

        let mut vm = VirtualMachine::with_id(&lua, record.id);
        vm.attach_nic(nic);
        vms.push(vm);

        println!(
            "[cluster]   Created: {} ({}) - owner: {}",
            record.hostname,
            record.ip.as_deref().unwrap_or("no ip"),
            haru.username
        );
    } else {
        // Restore only Haru's VM
        let haru_vm_record = existing_vms
            .into_iter()
            .find(|v| v.owner_id == Some(haru.id))
            .expect("Haru's VM should exist");

        let mut vm = VirtualMachine::with_id(&lua, haru_vm_record.id);

        // Re-attach NIC if VM had an IP
        if let (Some(ip_str), Some(subnet_str)) = (&haru_vm_record.ip, &haru_vm_record.subnet) {
            if let (Some(ip), Some(subnet)) = (Ipv4Addr::parse(ip_str), Subnet::parse(subnet_str))
            {
                let nic = net::nic::NIC::new(ip, subnet);
                vm.attach_nic(nic);
            }
        }

        // Mark as running
        vm_service
            .set_status(haru_vm_record.id, "running")
            .await
            .expect("Failed to set VM status");

        // Register in NetManager
        if let Some(ip_str) = &haru_vm_record.ip {
            if let Some(ip) = Ipv4Addr::parse(ip_str) {
                manager.net_manager.register_vm(ip, haru_vm_record.id);
            }
        }

        // Register DNS if present
        if let (Some(dns_name), Some(ip_str)) = (&haru_vm_record.dns_name, &haru_vm_record.ip) {
            if let Some(ip) = Ipv4Addr::parse(ip_str) {
                manager.dns.register_a(dns_name, ip);
            }
        }

        vms.push(vm);

        println!(
            "[cluster]   Restored: {} ({}) - owner: {}",
            haru_vm_record.hostname,
            haru_vm_record.ip.as_deref().unwrap_or("no ip"),
            haru.username
        );
    }

    println!("[cluster] Ready. {} VM active (Haru's VM). Starting game loop and gRPC server...", vms.len());

    let terminal_hub = new_hub();

    // ── gRPC server (runs in background task) ──
    let grpc_addr = GRPC_ADDR.parse().expect("Invalid gRPC address");
    let game_svc = ClusterGameService::new(
        player_service.clone(),
        vm_service.clone(),
        fs_service.clone(),
        user_service.clone(),
        faction_service.clone(),
        terminal_hub.clone(),
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
    manager.run_loop(&lua, &mut vms, terminal_hub).await;
}
