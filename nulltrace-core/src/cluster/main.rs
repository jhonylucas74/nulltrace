mod auth;
mod bench_scripts;
mod bin_programs;
mod file_search;
mod path_util;
mod process;
mod process_parser;
mod vm;
mod os;
mod net;
mod db;
mod grpc;
mod lua_api;
mod process_run_hub;
mod process_spy_hub;
mod terminal_hub;
mod vm_manager;
mod vm_worker;

use dashmap::DashMap;
use db::faction_service::FactionService;
use db::fs_service::FsService;
use db::player_service::PlayerService;
use db::shortcuts_service::ShortcutsService;
use db::user_service::UserService;
use db::vm_service::VmService;
use grpc::game::game_service_server::GameServiceServer;
use grpc::ClusterGameService;
use process_run_hub::new_hub as new_process_run_hub;
use process_spy_hub::new_hub as new_process_spy_hub;
use terminal_hub::new_hub;
use net::ip::{Ipv4Addr, Subnet};
use lua_api::context::VmContext;
use std::sync::Arc;
use tonic::transport::Server;
use tonic_web::GrpcWebLayer;
use vm::VirtualMachine;
use vm_manager::{ProcessSnapshot, VmManager};
use db::vm_service::VmConfig;

/// Creates a fully configured Lua state for a VM: sandbox, APIs, VmContext, memory limit.
fn create_vm_lua_state(
    pool: sqlx::PgPool,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
) -> Result<mlua::Lua, mlua::Error> {
    let lua = os::create_lua_state();
    lua.set_app_data(VmContext::new(pool));
    lua_api::register_all(&lua, fs_service, user_service)?;
    lua.set_memory_limit(os::LUA_MEMORY_LIMIT_BYTES)?;
    Ok(lua)
}

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
    let shortcuts_service = Arc::new(ShortcutsService::new(pool.clone()));

    // ── Seed default player (Haru) if not present ──
    player_service
        .seed_haru()
        .await
        .expect("Failed to seed default player");
    println!("[cluster] Default player ready (Haru)");

    // ── Seed webserver player (Nexus VM owner) if not present ──
    player_service
        .seed_webserver()
        .await
        .expect("Failed to seed webserver player");

    println!("[cluster] Lua APIs registered per VM (fs, net, os)");

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
        create_stress_vms(pool.clone(), fs_service.clone(), user_service.clone())
    } else {
        load_game_vms(&mut manager, pool.clone(), fs_service.clone(), user_service.clone(), &player_service, &vm_service).await
    };

    let terminal_hub = new_hub();
    let process_spy_hub = new_process_spy_hub();
    let process_run_hub = new_process_run_hub();

    let process_snapshot_store: Arc<DashMap<uuid::Uuid, Vec<ProcessSnapshot>>> =
        Arc::new(DashMap::new());
    let vm_lua_memory_store: Arc<DashMap<uuid::Uuid, u64>> = Arc::new(DashMap::new());

    // ── gRPC server (runs in background task) ──
    let grpc_addr = GRPC_ADDR.parse().expect("Invalid gRPC address");
    let game_svc = ClusterGameService::new(
        player_service.clone(),
        vm_service.clone(),
        fs_service.clone(),
        user_service.clone(),
        faction_service.clone(),
        shortcuts_service.clone(),
        terminal_hub.clone(),
        process_spy_hub.clone(),
        process_run_hub.clone(),
        process_snapshot_store.clone(),
        vm_lua_memory_store.clone(),
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
        .run_loop(&mut vms, terminal_hub, process_spy_hub, process_run_hub, process_snapshot_store, vm_lua_memory_store, &pool, stress_mode)
        .await;
}

/// Stress scenario: which program/script to run in each VM.
enum StressScenario {
    /// Simple CPU loop: print tick count (baseline).
    SimpleLoop,
    /// mem_stress: allocates memory until 1 MB limit, triggers VM reset.
    MemStress,
    /// coin: random flips, bounded history, CPU + some memory.
    Coin,
    /// FS stress: repeated fs.ls("/") to stress filesystem API.
    FsListLoop,
    /// CPU + table stress: loop with string concat and table ops (bounded memory).
    CpuTableLoop,
}

/// Criar 5k VMs in-memory para stress test.
/// VMs are distributed across 5 scenarios to stress: CPU loop, mem_stress, coin, fs.ls, and table/string ops.
fn create_stress_vms(
    pool: sqlx::PgPool,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
) -> Vec<VirtualMachine> {
    const NUM_SCENARIOS: usize = 5;
    println!("[cluster] STRESS TEST MODE: Creating 5,000 VMs (in-memory), 5 scenarios...");
    let mut vms = Vec::new();
    let start_creation = std::time::Instant::now();

    for i in 0..5_000 {
        let lua = create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone())
            .expect("Failed to create VM Lua state");
        let mut vm = VirtualMachine::new(lua);

        let scenario = match i % NUM_SCENARIOS {
            0 => StressScenario::SimpleLoop,
            1 => StressScenario::MemStress,
            2 => StressScenario::Coin,
            3 => StressScenario::FsListLoop,
            _ => StressScenario::CpuTableLoop,
        };

        match scenario {
            StressScenario::SimpleLoop => {
                vm.os.spawn_process(
                    &vm.lua,
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
            }
            StressScenario::MemStress => {
                vm.os.spawn_process(
                    &vm.lua,
                    bin_programs::MEM_STRESS,
                    vec!["4".to_string()],
                    0,
                    "root",
                );
            }
            StressScenario::Coin => {
                vm.os.spawn_process(
                    &vm.lua,
                    bin_programs::COIN,
                    vec!["1000".to_string()],
                    0,
                    "root",
                );
            }
            StressScenario::FsListLoop => {
                vm.os.spawn_process(
                    &vm.lua,
                    r#"
local n = 0
while true do
    local entries = fs.ls("/")
    n = n + 1
    if n % 100 == 0 then
        io.write("fs_stress: " .. n .. " ls(/)\n")
    end
end
                    "#,
                    vec![],
                    0,
                    "root",
                );
            }
            StressScenario::CpuTableLoop => {
                vm.os.spawn_process(
                    &vm.lua,
                    r#"
local t = {}
local cap = 500
local count = 0
while true do
    count = count + 1
    t[#t + 1] = string.rep("x", 64)
    if #t > cap then table.remove(t, 1) end
    if count % 50 == 0 then
        io.write("table_stress: " .. count .. " iterations\n")
    end
end
                    "#,
                    vec![],
                    0,
                    "root",
                );
            }
        }

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
    println!("[cluster] Scenarios: SimpleLoop | MemStress | Coin | FsListLoop | CpuTableLoop (1k VMs each)");
    println!("[cluster] Starting stress test...\n");

    vms
}

/// Carregar VMs do banco de dados para modo jogo
async fn load_game_vms(
    manager: &mut VmManager,
    pool: sqlx::PgPool,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    player_service: &Arc<PlayerService>,
    vm_service: &Arc<VmService>,
) -> Vec<VirtualMachine> {
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

    // 1b. Ensure Nexus (webserver) has a VM (create if doesn't exist)
    let webserver = player_service
        .get_by_username(db::player_service::WEBSERVER_USERNAME)
        .await
        .expect("Failed to get webserver")
        .expect("Webserver not found");

    let nexus_vm = vm_service
        .get_vm_by_owner_id(webserver.id)
        .await
        .expect("Failed to check webserver's VM");

    if nexus_vm.is_none() {
        println!("[cluster] Creating VM for ntml.org (webserver)...");
        let config = VmConfig {
            hostname: "ntml-server".to_string(),
            dns_name: Some("ntml.org".to_string()),
            cpu_cores: 2,
            memory_mb: 2048,
            disk_mb: 20480,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: Some(webserver.id),
        };
        let (record, _) = manager.create_vm(config).await.expect("Failed to create ntml.org VM");
        fs_service
            .mkdir(record.id, "/var/www", "root")
            .await
            .expect("Failed to create /var/www for ntml.org");
        let index_ntml = r##"Container:
  style:
    padding: 24
    backgroundColor: "#1a1a2e"
    minHeight: 400
  children:
    - Column:
        gap: 24
        style:
          width: 100%
        children:
          - Container:
              style:
                padding: 20
                backgroundColor: "#6b4cdf"
                borderRadius: 8
              children:
                - Text:
                    text: "What is NTML"
                    style:
                      fontSize: 32
                      color: "#ffffff"
                      textAlign: center
                      fontWeight: 700
          - Text:
              text: "NTML (NullTrace Markup Language) is a secure, YAML-based UI description language for the NullTrace game. Type-safe, designer-friendly, and sandboxed—no script injection or XSS. Build interfaces with layout primitives (Flex, Grid), components (Text, Button, Input), and CSS-like styling."
              style:
                fontSize: 16
                color: "#e0e0e0"
                textAlign: center
                lineHeight: 1.6
          - Divider:
              orientation: horizontal
              style:
                backgroundColor: "#333333"
                height: 1
                marginVertical: 8
          - Text:
              text: "Documentation"
              style:
                fontSize: 18
                color: "#e0e0e0"
                fontWeight: 700
          - Row:
              gap: 12
              wrap: true
              style:
                width: 100%
              children:
                - Link:
                    href: "/components"
                    children:
                      - Text:
                          text: "Components"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/styling"
                    children:
                      - Text:
                          text: "Styling"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/document-format"
                    children:
                      - Text:
                          text: "Document Format"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/examples"
                    children:
                      - Text:
                          text: "Examples"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/lua-api"
                    children:
                      - Text:
                          text: "Lua API"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/best-practices"
                    children:
                      - Text:
                          text: "Best Practices"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
                - Link:
                    href: "/http"
                    children:
                      - Text:
                          text: "HTTP"
                          style:
                            fontSize: 14
                            color: "#6b4cdf"
          - Link:
              href: "/robot.txt"
              target: new
              children:
                - Text:
                    text: "robot.txt (new tab)"
                    style:
                      fontSize: 12
                      color: "#a0a0a0"
"##;
        let notfound_ntml = r##"Container:
  style:
    padding: 24
    backgroundColor: "#1a1a2e"
  children:
    - Text:
        text: "Page not found"
        style:
          fontSize: 24
          color: "#e0e0e0"
          fontWeight: 700
    - Text:
        text: "The requested page does not exist at ntml.org."
        style:
          fontSize: 14
          color: "#a0a0a0"
    - Link:
        href: "/"
        children:
          - Text:
              text: "Back to home"
              style:
                fontSize: 14
                color: "#6b4cdf"
                marginTop: 16
"##;
        let components_ntml = include_str!("../../ntml_site/components.ntml");
        let styling_ntml = include_str!("../../ntml_site/styling.ntml");
        let document_format_ntml = include_str!("../../ntml_site/document-format.ntml");
        let examples_ntml = include_str!("../../ntml_site/examples.ntml");
        let lua_api_ntml = include_str!("../../ntml_site/lua-api.ntml");
        let best_practices_ntml = include_str!("../../ntml_site/best-practices.ntml");
        let http_ntml = include_str!("../../ntml_site/http.ntml");
        fs_service
            .write_file(record.id, "/var/www/index.ntml", index_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write index.ntml");
        fs_service
            .write_file(record.id, "/var/www/robot.txt", b"Robot: operational\n", Some("text/plain"), "root")
            .await
            .expect("Failed to write robot.txt");
        fs_service
            .write_file(record.id, "/var/www/components.ntml", components_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write components.ntml");
        fs_service
            .write_file(record.id, "/var/www/styling.ntml", styling_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write styling.ntml");
        fs_service
            .write_file(record.id, "/var/www/document-format.ntml", document_format_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write document-format.ntml");
        fs_service
            .write_file(record.id, "/var/www/examples.ntml", examples_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write examples.ntml");
        fs_service
            .write_file(record.id, "/var/www/lua-api.ntml", lua_api_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write lua-api.ntml");
        fs_service
            .write_file(record.id, "/var/www/best-practices.ntml", best_practices_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write best-practices.ntml");
        fs_service
            .write_file(record.id, "/var/www/http.ntml", http_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write http.ntml");
        fs_service
            .write_file(record.id, "/var/www/404.ntml", notfound_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await
            .expect("Failed to write 404.ntml");
        fs_service
            .write_file(record.id, "/etc/bootstrap", b"httpd /var/www\n", None, "root")
            .await
            .expect("Failed to write /etc/bootstrap for ntml.org");
        println!("[cluster] ✓ ntml.org VM created");

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
        let cpu_cores = manager
            .get_active_vm(vm_id)
            .map(|a| a.cpu_cores)
            .unwrap_or(1);
        let lua = create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone())
            .expect("Failed to create VM Lua state");
        let mut vm = VirtualMachine::with_id_and_cpu(lua, vm_id, cpu_cores);

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

        // Run bootstrap: read /etc/bootstrap and spawn each line
        if let Ok(Some((data, _))) = fs_service.read_file(vm_id, "/etc/bootstrap").await {
            if let Ok(content) = String::from_utf8(data) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(program) = parts.first() {
                        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                        if let Ok(Some((code, _))) =
                            fs_service.read_file(vm_id, &format!("/bin/{}", program)).await
                        {
                            if let Ok(lua_code) = String::from_utf8(code) {
                                vm.os.spawn_process(&vm.lua, &lua_code, args, 0, "root");
                            }
                        }
                    }
                }
            }
        }

        vms.push(vm);
    }

    println!("[cluster] ✓ Loaded {} VMs from database", vms.len());
    vms
}
