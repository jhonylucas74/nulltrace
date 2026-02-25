mod auth;
mod bench_scripts;
mod bin_programs;
mod file_search;
mod incoming_money_listener;
mod mailbox_hub;
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
mod sites;
mod vm_manager;
mod vm_worker;

use dashmap::DashMap;
use db::email_account_service::EmailAccountService;
use db::email_service::EmailService;
use db::faction_service::FactionService;
use db::fs_service::FsService;
use db::player_service::PlayerService;
use db::shortcuts_service::ShortcutsService;
use db::user_service::UserService;
use db::vm_service::VmService;
use db::wallet_service::WalletService;
use db::wallet_card_service::WalletCardService;
use db::card_invoice_service::CardInvoiceService;
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
use db::wallet_common::{generate_btc_address, generate_eth_address, generate_sol_address};
use uuid::Uuid;

/// Creates a fully configured Lua state for a VM: sandbox, APIs, VmContext, memory limit.
/// When fkebank_service and crypto_service are Some, the fkebank, crypto, and incoming_money Lua APIs are registered (for NPC VMs like money.null).
/// When card_invoice_service is Some, the card API is registered (for NPC VMs like card.null).
fn create_vm_lua_state(
    pool: sqlx::PgPool,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: mailbox_hub::MailboxHub,
    fkebank_service: Option<Arc<db::fkebank_account_service::FkebankAccountService>>,
    crypto_service: Option<Arc<db::crypto_wallet_service::CryptoWalletService>>,
    incoming_money_listener: Option<Arc<incoming_money_listener::IncomingMoneyListener>>,
    card_invoice_service: Option<Arc<CardInvoiceService>>,
) -> Result<mlua::Lua, mlua::Error> {
    let lua = os::create_lua_state();
    lua.set_app_data(VmContext::new(pool));
    lua_api::register_all(
        &lua,
        fs_service,
        user_service,
        email_service,
        email_account_service,
        mailbox_hub,
        fkebank_service,
        crypto_service,
        incoming_money_listener,
        card_invoice_service,
    )?;
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
    let email_service = Arc::new(EmailService::new(pool.clone()));
    let email_account_service = Arc::new(EmailAccountService::new(pool.clone()));
    let wallet_service = Arc::new(WalletService::new(pool.clone()));
    let wallet_card_service = Arc::new(WalletCardService::new(pool.clone()));
    let card_invoice_service = Arc::new(CardInvoiceService::new(
        pool.clone(),
        wallet_service.fkebank_service(),
        wallet_card_service.clone(),
    ));
    let mailbox_hub = mailbox_hub::new_hub();

    // ── Seed default player (Haru) if not present ──
    player_service
        .seed_haru()
        .await
        .expect("Failed to seed default player");
    println!("[cluster] Default player ready (Haru)");

    // Seed Haru's wallet (idempotent) and give $20 initial USD if balance is zero
    if let Some(haru) = player_service.get_by_username("Haru").await.ok().flatten() {
        let _ = wallet_service.create_wallet_for_player(haru.id).await;
        let balances = wallet_service.get_balances(haru.id).await.ok().unwrap_or_default();
        let usd_balance = balances.iter().find(|b| b.currency == "USD").map(|b| b.balance).unwrap_or(0);
        if usd_balance == 0 {
            let _ = wallet_service.credit(haru.id, "USD", 2000, "Initial balance").await;
            println!("[cluster] Haru initial USD balance set to $20");
        }
    }

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
        email_account_service.clone(),
        email_service.clone(),
        mailbox_hub.clone(),
        subnet,
    );

    // ── Redis (optional — cluster works without it) ──
    match manager.net_manager.connect_redis(REDIS_URL) {
        Ok(()) => println!("[cluster] Connected to Redis"),
        Err(e) => println!("[cluster] Redis not available ({}), running without cross-pod support", e),
    }

    // ── IncomingMoneyListener (for money.null double-back; requires Redis) ──
    let incoming_money_listener = incoming_money_listener::IncomingMoneyListener::new(pool.clone(), REDIS_URL)
        .ok()
        .map(|l| {
            let arc = Arc::new(l);
            arc.clone().spawn_poll_loop(1000);
            arc
        });
    if incoming_money_listener.is_some() {
        println!("[cluster] IncomingMoneyListener started (poll every 1s)");
    }

    // ── Criar ou carregar VMs dependendo do modo ──
    let mut vms = if stress_mode {
        create_stress_vms(
            pool.clone(),
            fs_service.clone(),
            user_service.clone(),
            email_service.clone(),
            email_account_service.clone(),
            mailbox_hub.clone(),
        )
    } else {
        load_game_vms(
            &mut manager,
            pool.clone(),
            fs_service.clone(),
            user_service.clone(),
            email_service.clone(),
            email_account_service.clone(),
            mailbox_hub.clone(),
            &player_service,
            &vm_service,
            &wallet_service,
            &card_invoice_service,
            incoming_money_listener.clone(),
        )
        .await
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
        email_service.clone(),
        email_account_service.clone(),
        wallet_service.clone(),
        wallet_card_service.clone(),
        mailbox_hub.clone(),
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
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: mailbox_hub::MailboxHub,
) -> Vec<VirtualMachine> {
    const NUM_SCENARIOS: usize = 5;
    println!("[cluster] STRESS TEST MODE: Creating 5,000 VMs (in-memory), 5 scenarios...");
    let mut vms = Vec::new();
    let start_creation = std::time::Instant::now();

    for i in 0..5_000 {
        let lua = create_vm_lua_state(
            pool.clone(),
            fs_service.clone(),
            user_service.clone(),
            email_service.clone(),
            email_account_service.clone(),
            mailbox_hub.clone(),
            None,
            None,
            None,
            None,
        )
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
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: mailbox_hub::MailboxHub,
    player_service: &Arc<PlayerService>,
    vm_service: &Arc<VmService>,
    wallet_service: &Arc<WalletService>,
    card_invoice_service: &Arc<CardInvoiceService>,
    incoming_money_listener: Option<Arc<incoming_money_listener::IncomingMoneyListener>>,
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
            create_email_account: true,
        };
        manager.create_vm(config).await.expect("Failed to create Haru's VM");
        println!("[cluster] ✓ Haru's VM created");

        // Clear active_vms to avoid duplication when restore_vms() runs
        manager.clear_active_vms();
    }

    // 1b. Restore all running/crashed VMs first (player + NPC like money.null, emailbox.null)
    match manager.restore_vms().await {
        Ok(records) => {
            println!("[cluster] Restored {} VM(s) from database", records.len());
        }
        Err(e) => {
            println!("[cluster] Warning: Failed to restore VMs: {}", e);
            return vms;
        }
    }

    // 1c. Load site VMs from sites/ folder (destroy existing + create; sites are reset each start)
    let sites_base = sites::sites_base_path();
    if sites_base.exists() {
        if let Err(e) = sites::load_site_vms(
            manager,
            &fs_service,
            &player_service,
            &vm_service,
            &sites_base,
        )
        .await
        {
            println!("[cluster] Warning: load_site_vms failed: {}", e);
        }
    }

    // 1d. Create NPC VM emailbox.null only if not already restored
    let has_emailbox = manager
        .get_active_vms()
        .iter()
        .any(|v| v.dns_name.as_deref() == Some("emailbox.null"));
    if !has_emailbox {
        let config = VmConfig {
            hostname: "emailbox".to_string(),
            dns_name: Some("emailbox.null".to_string()),
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
            create_email_account: true,
        };
        let (record, _nic) = manager
            .create_vm(config)
            .await
            .expect("Failed to create emailbox NPC VM");
        let vm_id = record.id;

        let _ = fs_service.mkdir(vm_id, "/var/www", "root").await;
        let body_content = b"Hello from the emailbox NPC. This is a test message.";
        let _ = fs_service
            .write_file(vm_id, "/var/www/body.txt", body_content, Some("text/plain"), "root")
            .await;
        let index_header = b"# Sent emails (to=... subject=... n=...)\n";
        let _ = fs_service
            .write_file(vm_id, "/var/www/index", index_header, Some("text/plain"), "root")
            .await;

        let emailbox_httpd_lua = include_str!("../lua_scripts/emailbox_httpd.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/emailbox_httpd.lua",
                emailbox_httpd_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        let email_sender_lua = include_str!("../lua_scripts/email_sender.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/email_sender.lua",
                email_sender_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        let bootstrap_content = "lua /var/www/emailbox_httpd.lua\nlua /var/www/email_sender.lua\n";
        let _ = fs_service
            .write_file(
                vm_id,
                "/etc/bootstrap",
                bootstrap_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        println!("[cluster] ✓ NPC VM emailbox.null created (emailbox_httpd + email_sender)");
    }

    // 1e. Create NPC VM money.null only if not already restored
    let has_money = manager
        .get_active_vms()
        .iter()
        .any(|v| v.dns_name.as_deref() == Some("money.null"));
    if !has_money {
        let config = VmConfig {
            hostname: "money".to_string(),
            dns_name: Some("money.null".to_string()),
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
            create_email_account: false,
        };
        let (record, _nic) = manager
            .create_vm(config)
            .await
            .expect("Failed to create money NPC VM");
        let vm_id = record.id;

        let (key, token) = wallet_service
            .create_wallet_for_account("money.null", Some("Money Null"), None)
            .await
            .expect("Failed to create wallet for money.null");

        // Seed money.null with 100 USD ($100 = 10000 cents) if balance is zero
        let fkebank = wallet_service.fkebank_service();
        if fkebank.get_balance_by_key(&key).await.unwrap_or(0) == 0 {
            let _ = fkebank.credit(&key, 10_000, "seed").await;
        }

        let _ = fs_service.mkdir(vm_id, "/etc/wallet", "root").await;
        let _ = fs_service.mkdir(vm_id, "/etc/wallet/fkebank", "root").await;
        let _ = fs_service.mkdir(vm_id, "/etc/wallet/keys", "root").await;
        let _ = fs_service.mkdir(vm_id, "/var/www", "root").await;

        // Seed crypto (100 of each): generate addresses, register, write .priv, credit, write crypto_addresses
        let crypto = wallet_service.crypto_service();
        let mut crypto_lines = Vec::new();
        for (currency, addr) in [
            ("BTC", generate_btc_address()),
            ("ETH", generate_eth_address()),
            ("SOL", generate_sol_address()),
        ] {
            let _ = crypto.register(&addr, None, currency).await;
            let priv_content = format!("{}", Uuid::new_v4().simple());
            let priv_path = format!("/etc/wallet/keys/{}.priv", currency.to_lowercase());
            let _ = fs_service
                .write_file(vm_id, &priv_path, priv_content.as_bytes(), Some("text/plain"), "root")
                .await;
            let _ = crypto.credit(currency, &addr, 10_000, "seed", "system").await;
            crypto_lines.push(format!("{}={}", currency, addr));
        }
        let crypto_addrs_content = crypto_lines.join("\n");
        let _ = fs_service
            .write_file(
                vm_id,
                "/etc/wallet/crypto_addresses",
                crypto_addrs_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        let _ = fs_service
            .write_file(vm_id, "/etc/wallet/fkebank/token", token.as_bytes(), Some("text/plain"), "root")
            .await;

        let money_init_lua = include_str!("../lua_scripts/money_init.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/money_init.lua",
                money_init_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;
        let money_httpd_lua = include_str!("../lua_scripts/money_httpd.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/money_httpd.lua",
                money_httpd_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;
        let money_refund_lua = include_str!("../lua_scripts/money_refund.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/money_refund.lua",
                money_refund_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        let bootstrap_content = "lua /var/www/money_init.lua\nlua /var/www/money_httpd.lua\nlua /var/www/money_refund.lua\n";
        let _ = fs_service
            .write_file(
                vm_id,
                "/etc/bootstrap",
                bootstrap_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        println!("[cluster] ✓ NPC VM money.null created (Fkebank account + httpd + double-back)");
    }

    // 1f. Create NPC VM card.null only if not already restored
    let has_card = manager
        .get_active_vms()
        .iter()
        .any(|v| v.dns_name.as_deref() == Some("card.null"));
    if !has_card {
        let config = VmConfig {
            hostname: "card".to_string(),
            dns_name: Some("card.null".to_string()),
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
            create_email_account: false,
        };
        let (record, _nic) = manager
            .create_vm(config)
            .await
            .expect("Failed to create card NPC VM");
        let vm_id = record.id;

        let (_key, token) = wallet_service
            .create_wallet_for_account("card.null", Some("Card Null"), None)
            .await
            .expect("Failed to create wallet for card.null");

        let _ = fs_service.mkdir(vm_id, "/etc/wallet", "root").await;
        let _ = fs_service.mkdir(vm_id, "/etc/wallet/fkebank", "root").await;
        let _ = fs_service.mkdir(vm_id, "/var/www", "root").await;
        let _ = fs_service.mkdir(vm_id, "/var/www/scripts", "root").await;

        let _ = fs_service
            .write_file(vm_id, "/etc/wallet/fkebank/token", token.as_bytes(), Some("text/plain"), "root")
            .await;

        let card_index_ntml = include_str!("../lua_scripts/card_index.ntml");
        let _ = fs_service
            .write_file(vm_id, "/var/www/index.ntml", card_index_ntml.as_bytes(), Some("application/x-ntml"), "root")
            .await;

        let card_scripts_card_lua = include_str!("../lua_scripts/card_scripts_card.lua");
        let _ = fs_service
            .write_file(vm_id, "/var/www/scripts/card.lua", card_scripts_card_lua.as_bytes(), Some("text/plain"), "root")
            .await;

        let card_httpd_lua = include_str!("../lua_scripts/card_httpd.lua");
        let _ = fs_service
            .write_file(
                vm_id,
                "/var/www/card_httpd.lua",
                card_httpd_lua.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        let bootstrap_content = "lua /var/www/card_httpd.lua\n";
        let _ = fs_service
            .write_file(
                vm_id,
                "/etc/bootstrap",
                bootstrap_content.as_bytes(),
                Some("text/plain"),
                "root",
            )
            .await;

        println!("[cluster] ✓ NPC VM card.null created (card invoice HTTP site)");
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
        let is_card_null = manager
            .get_active_vm(vm_id)
            .map(|v| v.dns_name.as_deref() == Some("card.null"))
            .unwrap_or(false);
        let card_svc = if is_card_null {
            Some(card_invoice_service.clone())
        } else {
            None
        };
        let lua = create_vm_lua_state(
            pool.clone(),
            fs_service.clone(),
            user_service.clone(),
            email_service.clone(),
            email_account_service.clone(),
            mailbox_hub.clone(),
            Some(wallet_service.fkebank_service()),
            Some(wallet_service.crypto_service()),
            incoming_money_listener.clone(),
            card_svc,
        )
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

        // For money.null VM: always overwrite scripts with latest embedded versions on cluster start
        if let Some(active_vm) = manager.get_active_vm(vm_id) {
            if active_vm.dns_name.as_deref() == Some("money.null") {
                let money_refund_lua = include_str!("../lua_scripts/money_refund.lua");
                let _ = fs_service
                    .write_file(
                        vm_id,
                        "/var/www/money_refund.lua",
                        money_refund_lua.as_bytes(),
                        Some("text/plain"),
                        "root",
                    )
                    .await;
                println!("[cluster]   money.null: updated money_refund.lua to latest version");
            }
            if active_vm.dns_name.as_deref() == Some("card.null") {
                let _ = fs_service.mkdir(vm_id, "/var/www/scripts", "root").await;
                let card_index_ntml = include_str!("../lua_scripts/card_index.ntml");
                let _ = fs_service
                    .write_file(vm_id, "/var/www/index.ntml", card_index_ntml.as_bytes(), Some("application/x-ntml"), "root")
                    .await;
                let card_scripts_card_lua = include_str!("../lua_scripts/card_scripts_card.lua");
                let _ = fs_service
                    .write_file(vm_id, "/var/www/scripts/card.lua", card_scripts_card_lua.as_bytes(), Some("text/plain"), "root")
                    .await;
                let card_httpd_lua = include_str!("../lua_scripts/card_httpd.lua");
                let _ = fs_service
                    .write_file(
                        vm_id,
                        "/var/www/card_httpd.lua",
                        card_httpd_lua.as_bytes(),
                        Some("text/plain"),
                        "root",
                    )
                    .await;
                println!("[cluster]   card.null: updated card_httpd.lua, index.ntml, scripts/card.lua to latest version");
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
                                let arg0 = args.first().map(|s| s.as_str()).unwrap_or("");
                                if arg0.contains("money_refund") {
                                    println!("[cluster] Spawning money_refund for vm {} (hostname={})", vm_id, manager.get_active_vm(vm_id).map(|a| a.hostname.as_str()).unwrap_or("?"));
                                }
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
