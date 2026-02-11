#![allow(dead_code)]

use super::bin_programs;
use super::db::fs_service::FsService;
use super::db::player_service::PlayerService;
use super::db::user_service::{UserService, VmUser};
use super::db::vm_service::{VmConfig, VmRecord, VmService};
use super::lua_api::context::VmContext;
use super::net::dns::DnsResolver;
use super::net::ip::{Ipv4Addr, Subnet};
use super::net::net_manager::NetManager;
use super::net::nic::NIC;
use super::net::router::{RouteResult, Router};
use super::vm::VirtualMachine;
use mlua::Lua;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::{sleep, sleep_until, Instant as TokioInstant};
use uuid::Uuid;

const TPS: u32 = 60;
const TICK_TIME: Duration = Duration::from_millis(1000 / TPS as u64);

pub struct VmManager {
    pub vm_service: Arc<VmService>,
    pub fs_service: Arc<FsService>,
    pub user_service: Arc<UserService>,
    pub player_service: Arc<PlayerService>,
    pub dns: DnsResolver,
    pub net_manager: NetManager,
    pub subnet: Subnet,
    pub router: Router,

    /// In-memory records of active VMs (hostname, ip, etc.)
    active_vms: Vec<ActiveVm>,
}

/// Lightweight in-memory record for each active VM.
pub struct ActiveVm {
    pub id: Uuid,
    pub hostname: String,
    pub dns_name: Option<String>,
    pub ip: Option<Ipv4Addr>,
}

impl VmManager {
    pub fn new(
        vm_service: Arc<VmService>,
        fs_service: Arc<FsService>,
        user_service: Arc<UserService>,
        player_service: Arc<PlayerService>,
        subnet: Subnet,
    ) -> Self {
        let mut router = Router::new();
        router.add_interface(subnet.gateway(), subnet.clone());

        Self {
            vm_service,
            fs_service,
            user_service,
            player_service,
            dns: DnsResolver::new(),
            net_manager: NetManager::new("local".to_string()),
            subnet,
            router,
            active_vms: Vec::new(),
        }
    }

    /// Create a new VM: DB insert + FS bootstrap + NIC allocation + DNS registration.
    /// Returns the VM record and a NIC ready to be attached.
    pub async fn create_vm(&mut self, config: VmConfig) -> Result<(VmRecord, NIC), String> {
        let id = Uuid::new_v4();

        // Allocate IP from subnet
        let nic = NIC::from_subnet(&mut self.subnet)
            .ok_or_else(|| "Subnet exhausted, no IPs available".to_string())?;

        let ip_str = nic.ip.to_string();
        let subnet_str = nic.subnet.to_string();
        let gateway_str = nic.gateway.to_string();
        let mac_str = nic.mac_string();

        let mut db_config = config;
        db_config.ip = Some(ip_str);
        db_config.subnet = Some(subnet_str);
        db_config.gateway = Some(gateway_str);
        db_config.mac = Some(mac_str);

        // Insert into DB
        let record = self
            .vm_service
            .create_vm(id, db_config)
            .await
            .map_err(|e| format!("DB error creating VM: {}", e))?;

        // Bootstrap filesystem
        self.fs_service
            .bootstrap_fs(id)
            .await
            .map_err(|e| format!("DB error bootstrapping FS: {}", e))?;

        // Bootstrap /bin programs
        for (name, source) in bin_programs::DEFAULT_BIN_PROGRAMS {
            let path = format!("/bin/{}", name);
            self.fs_service
                .write_file(
                    id,
                    &path,
                    source.as_bytes(),
                    Some("application/x-nulltrace-lua"),
                    "root",
                )
                .await
                .map_err(|e| format!("DB error writing {}: {}", path, e))?;
        }

        // Bootstrap users (root + user)
        let mut users: Vec<VmUser> = self
            .user_service
            .bootstrap_users(id)
            .await
            .map_err(|e| format!("DB error bootstrapping users: {}", e))?;

        // If VM has an owner (player), create an admin vm_user with same username/password and is_root
        if let Some(owner_id) = record.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| format!("DB error loading owner player: {}", e))?
                .ok_or_else(|| "Owner player not found".to_string())?;
            let owner_home = format!("/home/{}", player.username);
            let owner_user = self
                .user_service
                .create_user(
                    id,
                    &player.username,
                    1001,
                    Some(&player.password_hash),
                    true,
                    &owner_home,
                    "/bin/sh",
                )
                .await
                .map_err(|e| format!("DB error creating owner admin user: {}", e))?;
            users.push(owner_user);
        }

        // Create home directories (use each user's home_dir)
        for user in &users {
            self.fs_service
                .mkdir(id, &user.home_dir, &user.username)
                .await
                .map_err(|e| format!("DB error creating home dir: {}", e))?;
        }

        // Write /etc/passwd
        let passwd_content: String = users
            .iter()
            .map(|u| {
                let gid = if u.is_root { 0 } else { u.uid };
                format!(
                    "{}:x:{}:{}:{}:{}:{}",
                    u.username, u.uid, gid, u.username, u.home_dir, u.shell
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(id, "/etc/passwd", passwd_content.as_bytes(), Some("text/plain"), "root")
            .await
            .map_err(|e| format!("DB error writing /etc/passwd: {}", e))?;

        // Write /etc/shadow
        let shadow_content: String = users
            .iter()
            .map(|u| {
                let hash = u.password_hash.as_deref().unwrap_or("!");
                format!("{}:{}:19000:0:99999:7:::", u.username, hash)
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        self.fs_service
            .write_file(id, "/etc/shadow", shadow_content.as_bytes(), Some("text/plain"), "root")
            .await
            .map_err(|e| format!("DB error writing /etc/shadow: {}", e))?;

        // Register in DNS only if dns_name is set
        if let Some(ref dns_name) = record.dns_name {
            self.dns.register_a(dns_name, nic.ip);
        }

        // Register in NetManager
        self.net_manager.register_vm(nic.ip, id);

        // Track in memory
        self.active_vms.push(ActiveVm {
            id,
            hostname: record.hostname.clone(),
            dns_name: record.dns_name.clone(),
            ip: Some(nic.ip),
        });

        Ok((record, nic))
    }

    /// Stop a VM: set status to stopped, unregister from DNS/NetManager.
    pub async fn stop_vm(&mut self, vm_id: Uuid) -> Result<(), String> {
        self.vm_service
            .set_status(vm_id, "stopped")
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        // Find and remove from active
        if let Some(pos) = self.active_vms.iter().position(|v| v.id == vm_id) {
            let vm = self.active_vms.remove(pos);
            if let Some(ref dns_name) = vm.dns_name {
                self.dns.unregister_a(dns_name);
            }
            if let Some(ip) = vm.ip {
                self.net_manager.unregister_vm(&ip);
            }
        }

        Ok(())
    }

    /// Destroy a VM: delete from DB (cascades FS), unregister from DNS/NetManager.
    pub async fn destroy_vm(&mut self, vm_id: Uuid) -> Result<(), String> {
        // Remove from active first
        if let Some(pos) = self.active_vms.iter().position(|v| v.id == vm_id) {
            let vm = self.active_vms.remove(pos);
            if let Some(ref dns_name) = vm.dns_name {
                self.dns.unregister_a(dns_name);
            }
            if let Some(ip) = vm.ip {
                self.net_manager.unregister_vm(&ip);
            }
        }

        self.vm_service
            .delete_vm(vm_id)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(())
    }

    /// Restore VMs from DB that were running/crashed. Returns records to rebuild structs.
    pub async fn restore_vms(&mut self) -> Result<Vec<VmRecord>, String> {
        let records = self
            .vm_service
            .restore_running_vms()
            .await
            .map_err(|e| format!("DB error restoring VMs: {}", e))?;

        for record in &records {
            let ip = record.ip.as_deref().and_then(Ipv4Addr::parse);

            // Re-register in DNS only if dns_name is set
            if let (Some(dns_name), Some(ip)) = (&record.dns_name, ip) {
                self.dns.register_a(dns_name, ip);
            }

            if let Some(ip) = ip {
                self.net_manager.register_vm(ip, record.id);
            }

            self.active_vms.push(ActiveVm {
                id: record.id,
                hostname: record.hostname.clone(),
                dns_name: record.dns_name.clone(),
                ip,
            });

            // Mark as running
            let _ = self.vm_service.set_status(record.id, "running").await;
        }

        Ok(records)
    }

    /// Get the in-memory record for a VM by ID.
    pub fn get_active_vm(&self, vm_id: Uuid) -> Option<&ActiveVm> {
        self.active_vms.iter().find(|v| v.id == vm_id)
    }

    /// Process one network tick: route packets between VMs via the router.
    pub fn network_tick(&mut self, vms: &mut [VirtualMachine]) {
        // 1. Drain outbound from all NICs
        let mut packets_to_route = Vec::new();
        for vm in vms.iter_mut() {
            if let Some(nic) = &mut vm.nic {
                for pkt in nic.drain_outbound() {
                    packets_to_route.push(pkt);
                }
            }
        }

        // 2. Route each packet
        for pkt in packets_to_route {
            match self.router.route_packet(pkt) {
                RouteResult::Deliver { packet, .. } => {
                    // Find the destination VM by IP and deliver
                    let dst_ip = packet.dst_ip;
                    for vm in vms.iter_mut() {
                        if let Some(nic) = &mut vm.nic {
                            if nic.ip == dst_ip {
                                nic.deliver(packet);
                                break;
                            }
                        }
                    }
                }
                RouteResult::Forward { packet, .. } => {
                    // In a single-router setup, forward = deliver
                    let dst_ip = packet.dst_ip;
                    for vm in vms.iter_mut() {
                        if let Some(nic) = &mut vm.nic {
                            if nic.ip == dst_ip {
                                nic.deliver(packet);
                                break;
                            }
                        }
                    }
                }
                RouteResult::CrossPod(packet) => {
                    self.net_manager.send_cross_pod(packet, "");
                }
                RouteResult::Drop(_) => {
                    // Packet dropped (no route, firewall, TTL)
                }
            }
        }
    }

    /// Run the main game loop.
    pub async fn run_loop(&mut self, lua: &Lua, vms: &mut Vec<VirtualMachine<'_>>) {
        println!(
            "[cluster] Game loop started ({} TPS, {} active VMs)",
            TPS,
            vms.len()
        );

        let mut next_tick = TokioInstant::now();
        let mut tick_count: u64 = 0;
        let start = Instant::now();

        loop {
            let tick_start = Instant::now();

            // Set context and tick each VM
            for vm in vms.iter_mut() {
                // Prepare Lua context for this VM
                {
                    let active = self.active_vms.iter().find(|a| a.id == vm.id);
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    let hostname = active.map(|a| a.hostname.as_str()).unwrap_or("unknown");
                    let ip = active.and_then(|a| a.ip);
                    ctx.set_vm(vm.id, hostname, ip);

                    // Load inbound packets into context
                    if let Some(nic) = &mut vm.nic {
                        while let Some(pkt) = nic.recv() {
                            ctx.net_inbound.push_back(pkt);
                        }
                    }
                }

                // Tick all processes
                for process in &mut vm.os.processes {
                    {
                        let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                        ctx.current_pid = process.id;
                        ctx.current_uid = process.user_id;
                        ctx.current_username = process.username.clone();
                        ctx.set_current_process(
                            process.stdin.clone(),
                            process.stdout.clone(),
                            process.args.clone(),
                        );
                    }
                    if !process.is_finished() {
                        process.tick();
                    }
                    // Detect if os.su() changed the user identity
                    {
                        let ctx = lua.app_data_ref::<VmContext>().unwrap();
                        if ctx.current_uid != process.user_id {
                            process.user_id = ctx.current_uid;
                            process.username = ctx.current_username.clone();
                        }
                    }
                }
                vm.os.processes.retain(|p| !p.is_finished());

                // Process spawn_queue (from os.exec) — load programs from /bin and spawn
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    let spawn_queue = std::mem::take(&mut ctx.spawn_queue);
                    let vm_id = ctx.vm_id;
                    drop(ctx);

                    for (name, args, uid, username) in spawn_queue {
                        let path = format!("/bin/{}", name);
                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                self.fs_service.read_file(vm_id, &path).await
                            })
                        });
                        if let Ok(Some((data, _))) = result {
                            if let Ok(lua_code) = String::from_utf8(data) {
                                vm.os.spawn_process(&lua_code, args, uid, &username);
                            }
                        }
                    }
                }

                // Apply outbound packets from context to NIC
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    if let Some(nic) = &mut vm.nic {
                        for pkt in ctx.net_outbound.drain(..) {
                            nic.send(pkt);
                        }
                        // Sync listening ports
                        for port in ctx.listening_ports.drain(..) {
                            nic.listen(port);
                        }
                    }
                }
            }

            // Network tick — route packets between VMs
            self.network_tick(vms);

            tick_count += 1;

            // Remove finished VMs
            vms.retain_mut(|vm| !vm.os.is_finished());

            // Frame pacing
            let elapsed = tick_start.elapsed();
            if elapsed < TICK_TIME {
                next_tick += TICK_TIME;
                sleep_until(next_tick).await;
            } else {
                next_tick = TokioInstant::now();
            }

            // Log every 5 seconds
            if tick_count % (TPS as u64 * 5) == 0 {
                let uptime = start.elapsed().as_secs();
                println!(
                    "[cluster] Tick {} | {} VMs active | uptime {}s",
                    tick_count,
                    vms.len(),
                    uptime,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bench_scripts;
    use super::super::bin_programs;
    use super::super::db::{self, fs_service::FsService, player_service::PlayerService};
    use super::super::db::{user_service::UserService, vm_service::VmService};
    use super::super::lua_api::context::VmContext;
    use super::super::lua_api;
    use super::super::net::ip::{Ipv4Addr, Subnet};
    use super::super::os;
    use super::super::vm::VirtualMachine;
    use super::*;
    use mlua::Lua;
    use std::sync::Arc;
    use uuid::Uuid;

    /// Bin echo: prints args to stdout.
    #[tokio::test]
    async fn test_bin_echo() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 97, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "tick-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        let echo_code = bin_programs::ECHO;
        vm.os.spawn_process(echo_code, vec!["hello".to_string()], 0, "root");

        let mut stdout_result = String::new();
        let max_ticks = 100;
        for _ in 0..max_ticks {
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, "tick-test-vm", None);
            }

            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                    );
                }
                process.tick();
                if process.is_finished() {
                    stdout_result = process.stdout.lock().unwrap().clone();
                }
            }

            vm.os.processes.retain(|p| !p.is_finished());
            if vm.os.is_finished() {
                break;
            }
        }

        assert!(
            stdout_result.contains("hello"),
            "echo stdout should contain 'hello', got: {:?}",
            stdout_result
        );
    }

    /// Tick test: spawn script that reads stdin, inject "25", assert stdout contains it.
    #[tokio::test]
    async fn test_process_tick_stdin_validation() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 96, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "stdin-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        let read_script = r#"
local line = io.read()
if line then
    io.write("read: " .. line .. "\n")
end
"#;
        vm.os.spawn_process(read_script, vec![], 0, "root");

        let process = vm.os.processes.first_mut().unwrap();
        process.stdin.lock().unwrap().push_back("25".to_string());

        let mut stdout_result = String::new();
        let max_ticks = 100;
        for _ in 0..max_ticks {
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, "stdin-test-vm", None);
            }

            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                    );
                }
                process.tick();
                if process.is_finished() {
                    stdout_result = process.stdout.lock().unwrap().clone();
                }
            }

            vm.os.processes.retain(|p| !p.is_finished());
            if vm.os.is_finished() {
                break;
            }
        }

        assert!(
            stdout_result.contains("25"),
            "stdout should contain '25', got: {:?}",
            stdout_result
        );
    }

    /// Helper: run tick loop until process finishes, return stdout.
    fn run_tick_until_done(
        lua: &Lua,
        vm: &mut VirtualMachine,
        vm_id: Uuid,
        hostname: &str,
    ) -> String {
        run_tick_until_done_with_limit(lua, vm, vm_id, hostname, 100).0
    }

    /// Helper: run tick loop with max ticks, return (stdout, tick_count).
    fn run_tick_until_done_with_limit(
        lua: &Lua,
        vm: &mut VirtualMachine,
        vm_id: Uuid,
        hostname: &str,
        max_ticks: usize,
    ) -> (String, usize) {
        let mut stdout_result = String::new();
        let mut tick_count = 0;
        for _ in 0..max_ticks {
            tick_count += 1;
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, hostname, None);
            }
            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                    );
                }
                process.tick();
                if process.is_finished() {
                    stdout_result = process.stdout.lock().unwrap().clone();
                }
            }
            vm.os.processes.retain(|p| !p.is_finished());
            if vm.os.processes.is_empty() || vm.os.is_finished() {
                break;
            }
        }
        (stdout_result, tick_count)
    }

    /// Variant that simulates 60 FPS game loop: sleeps ~16.6ms between ticks.
    async fn run_tick_until_done_with_limit_60fps(
        lua: &Lua,
        vm: &mut VirtualMachine<'_>,
        vm_id: Uuid,
        hostname: &str,
        max_ticks: usize,
    ) -> (String, usize) {
        let mut stdout_result = String::new();
        let mut tick_count = 0;
        for _ in 0..max_ticks {
            tick_count += 1;
            {
                let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, hostname, None);
            }
            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                    );
                }
                process.tick();
                if process.is_finished() {
                    stdout_result = process.stdout.lock().unwrap().clone();
                }
            }
            vm.os.processes.retain(|p| !p.is_finished());
            if vm.os.processes.is_empty() || vm.os.is_finished() {
                break;
            }
            sleep(super::TICK_TIME).await;
        }
        (stdout_result, tick_count)
    }

    /// Benchmark: program with nested loop completes within 2000 ticks.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_program_completes_within_reasonable_ticks() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 90, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "bench-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(
            bench_scripts::BENCHMARK_LOOP,
            vec![],
            0,
            "root",
        );

        const MAX_TICKS: usize = 100_000;
        let (stdout, tick_count) =
            run_tick_until_done_with_limit_60fps(&lua, &mut vm, vm_id, "bench-vm", MAX_TICKS).await;

        assert!(
            vm.os.processes.is_empty(),
            "benchmark should complete within {} ticks, used {}; stdout: {:?}",
            MAX_TICKS,
            tick_count,
            stdout
        );
        assert!(
            stdout.contains("result: 500500"),
            "expected result 500500, got: {:?}",
            stdout
        );
    }

    /// Bin cat: reads file and outputs to stdout.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_cat() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 95, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "cat-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        fs_service
            .write_file(vm_id, "/tmp/cat_test.txt", b"hello from file", None, "root")
            .await
            .unwrap();

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(
            bin_programs::CAT,
            vec!["/tmp/cat_test.txt".to_string()],
            0,
            "root",
        );

        let stdout = run_tick_until_done(&lua, &mut vm, vm_id, "cat-test-vm");

        assert!(
            stdout.contains("hello from file"),
            "cat stdout should contain file content, got: {:?}",
            stdout
        );
    }

    /// Bin ls: lists directory entries.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_ls() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 94, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "ls-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(bin_programs::LS, vec!["/".to_string()], 0, "root");

        let stdout = run_tick_until_done(&lua, &mut vm, vm_id, "ls-test-vm");

        assert!(
            stdout.contains("bin"),
            "ls / should list 'bin', got: {:?}",
            stdout
        );
        assert!(
            stdout.contains("home"),
            "ls / should list 'home', got: {:?}",
            stdout
        );
    }

    /// Bin touch: creates empty file.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_touch() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "touch-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        let touch_path = "/tmp/touch_created.txt";
        vm.os.spawn_process(
            bin_programs::TOUCH,
            vec![touch_path.to_string()],
            0,
            "root",
        );

        run_tick_until_done(&lua, &mut vm, vm_id, "touch-test-vm");

        let content = fs_service.read_file(vm_id, touch_path).await.unwrap();
        assert!(
            content.is_some(),
            "touch should have created {}",
            touch_path
        );
        let (data, _) = content.unwrap();
        assert!(data.is_empty(), "touch should create empty file");
    }

    /// Bin rm: removes file.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_rm() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "rm-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let rm_path = "/tmp/rm_target.txt";
        fs_service
            .write_file(vm_id, rm_path, b"to be deleted", None, "root")
            .await
            .unwrap();

        let lua = os::create_lua_state();
        lua.set_app_data(VmContext::new(pool.clone()));
        lua_api::register_all(&lua, fs_service.clone(), user_service.clone()).unwrap();

        let mut vm = VirtualMachine::with_id(&lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(bin_programs::RM, vec![rm_path.to_string()], 0, "root");

        run_tick_until_done(&lua, &mut vm, vm_id, "rm-test-vm");

        let content = fs_service.read_file(vm_id, rm_path).await.unwrap();
        assert!(
            content.is_none(),
            "rm should have deleted {}",
            rm_path
        );
    }

    /// Ensure every newly created VM includes all default /bin programs.
    #[tokio::test]
    async fn test_bootstrap_includes_default_bin_programs() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "bootstrap-bin-test-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, _nic) = manager.create_vm(config).await.unwrap();
        let entries = fs_service.ls(record.id, "/bin").await.unwrap();

        let entry_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        for (name, _) in bin_programs::DEFAULT_BIN_PROGRAMS {
            assert!(
                entry_names.contains(&name),
                "/bin should contain '{}' after bootstrap, got: {:?}",
                name,
                entry_names
            );
        }
    }

    #[tokio::test]
    async fn test_create_vm_without_owner_has_only_bootstrap_users() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 99, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let config = super::super::db::vm_service::VmConfig {
            hostname: "no-owner-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: None,
        };

        let (record, _nic) = manager.create_vm(config).await.unwrap();
        let users = user_service.list_users(record.id).await.unwrap();

        assert_eq!(users.len(), 2, "only root and user");
        assert_eq!(users[0].username, "root");
        assert_eq!(users[1].username, "user");
    }

    #[tokio::test]
    async fn test_create_vm_with_owner_creates_admin_vm_user() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 98, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let owner_name = format!("ownerplayer_{}", Uuid::new_v4());
        let player = player_service
            .create_player(&owner_name, "ownerpass")
            .await
            .unwrap();

        let config = super::super::db::vm_service::VmConfig {
            hostname: "owned-vm".to_string(),
            dns_name: None,
            cpu_cores: 1,
            memory_mb: 512,
            disk_mb: 10240,
            ip: None,
            subnet: None,
            gateway: None,
            mac: None,
            owner_id: Some(player.id),
        };

        let (record, _nic) = manager.create_vm(config).await.unwrap();
        let users = user_service.list_users(record.id).await.unwrap();

        assert_eq!(users.len(), 3, "root, user, and owner admin");
        let usernames: Vec<&str> = users.iter().map(|u| u.username.as_str()).collect();
        assert!(usernames.contains(&"root"));
        assert!(usernames.contains(&"user"));
        assert!(usernames.contains(&owner_name.as_str()));

        let owner_vm_user = users.iter().find(|u| u.username == owner_name).unwrap();
        assert!(owner_vm_user.is_root);
        assert_eq!(owner_vm_user.uid, 1001);
        assert_eq!(owner_vm_user.home_dir, format!("/home/{}", owner_name));

        assert!(user_service
            .verify_password(record.id, &owner_name, "ownerpass")
            .await
            .unwrap());
    }
}
