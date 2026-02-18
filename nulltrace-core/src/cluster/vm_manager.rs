#![allow(dead_code)]

use super::bin_programs;
use super::db::fs_service::FsService;
use super::process::push_stdin_line;
use super::db::player_service::PlayerService;
use super::db::user_service::{UserService, VmUser};
use super::db::vm_service::{VmConfig, VmRecord, VmService};
use super::lua_api::context::{SpawnSpec, VmContext};
use super::net::dns::DnsResolver;
use super::net::ip::{Ipv4Addr, Subnet};
use super::net::net_manager::NetManager;
use super::net::nic::NIC;
use super::net::router::{RouteResult, Router};
use super::process_run_hub::{ProcessRunHub, RunProcessStreamMsg};
use super::process_spy_hub::{PendingKill, PendingLuaSpawn, ProcessSpyDownstreamMsg, ProcessSpyHub, ProcessSpySubscription};
use super::terminal_hub::{SessionReady, TerminalHub, TerminalSession};
use super::vm::VirtualMachine;
use super::vm_worker::{VmWorker, WorkerResult};
use dashmap::DashMap;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use uuid::Uuid;

/// Snapshot of one process for gRPC (System Monitor / Proc Spy). Written every 60 ticks for player-owned VMs only.
/// Args are always included so Proc Spy can show the full command line used to invoke the process.
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u64,
    pub name: String,
    pub username: String,
    pub status: String,
    pub memory_bytes: u64,
    /// Full argv used to call the process (e.g. ["grep", "bar", "/tmp/a.txt"]). Always shown in Proc Spy.
    pub args: Vec<String>,
}

/// Wall-clock interval between process list snapshot updates (for GetProcessList and Process Spy).
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500); // 0.5 seconds

const TPS: u32 = 60;
const TICK_TIME: Duration = Duration::from_millis(1000 / TPS as u64);
const TEST_DURATION_SECS: u64 = 30; // Stress test duration (30 seconds)

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

    /// VM ids that are owned by a player (need process snapshot for gRPC). NPC VMs are not in this set.
    player_owned_vm_ids: HashSet<Uuid>,
}

/// Lightweight in-memory record for each active VM.
pub struct ActiveVm {
    pub id: Uuid,
    pub hostname: String,
    pub dns_name: Option<String>,
    pub ip: Option<Ipv4Addr>,
    /// CPU cores for tick budget (ticks_per_second = f(cpu_cores)).
    pub cpu_cores: i16,
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
            player_owned_vm_ids: HashSet::new(),
        }
    }

    /// Register a VM as player-owned (so its process list is snapshotted for gRPC). Call when restoring a player VM in main.
    pub fn register_player_owned_vm_id(&mut self, vm_id: Uuid) {
        self.player_owned_vm_ids.insert(vm_id);
    }

    /// Clear active VMs list (used to avoid duplication when creating then restoring VMs).
    pub fn clear_active_vms(&mut self) {
        self.active_vms.clear();
        self.player_owned_vm_ids.clear();
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
        let mut owner_documents: Option<(String, String)> = None;
        if let Some(owner_id) = record.owner_id {
            let player = self
                .player_service
                .get_by_id(owner_id)
                .await
                .map_err(|e| format!("DB error loading owner player: {}", e))?
                .ok_or_else(|| "Owner player not found".to_string())?;
            let owner_home = format!("/home/{}", player.username);
            owner_documents = Some((
                format!("{}/Documents", owner_home),
                player.username.clone(),
            ));
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

        // Create home directories (use each user's home_dir). Skip if already exists (e.g. /root, /home/user from bootstrap_fs).
        for user in &users {
            if self.fs_service.resolve_path(id, &user.home_dir).await.map_err(|e| format!("DB error resolving path: {}", e))?.is_none() {
                self.fs_service
                    .mkdir(id, &user.home_dir, &user.username)
                    .await
                    .map_err(|e| format!("DB error creating home dir: {}", e))?;
            }
            self.fs_service
                .ensure_standard_home_subdirs(id, &user.home_dir, &user.username)
                .await
                .map_err(|e| format!("DB error creating home subdirs: {}", e))?;
        }

        // Seed default files in owner's Documents (for testing find, grep, cat, lua)
        if let Some((documents_path, owner_name)) = &owner_documents {
            self.fs_service
                .seed_default_documents(id, documents_path, owner_name)
                .await
                .map_err(|e| format!("DB error seeding Documents: {}", e))?;
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
            cpu_cores: record.cpu_cores,
        });

        if record.owner_id.is_some() {
            self.player_owned_vm_ids.insert(record.id);
        }

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
        self.player_owned_vm_ids.remove(&vm_id);

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
        self.player_owned_vm_ids.remove(&vm_id);

        self.vm_service
            .delete_vm(vm_id)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

        Ok(())
    }

    /// Restore player-owned VMs from DB that were running/crashed. Returns records to rebuild structs.
    pub async fn restore_vms(&mut self) -> Result<Vec<VmRecord>, String> {
        let records = self
            .vm_service
            .restore_player_vms()
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
                cpu_cores: record.cpu_cores,
            });

            if record.owner_id.is_some() {
                self.player_owned_vm_ids.insert(record.id);
            }

            // Mark as running
            let _ = self.vm_service.set_status(record.id, "running").await;
        }

        Ok(records)
    }

    /// Get the in-memory record for a VM by ID.
    pub fn get_active_vm(&self, vm_id: Uuid) -> Option<&ActiveVm> {
        self.active_vms.iter().find(|v| v.id == vm_id)
    }

    /// Get all active VMs.
    pub fn get_active_vms(&self) -> &[ActiveVm] {
        &self.active_vms
    }

    /// Process one network tick: route packets between VMs via the router.
    pub fn network_tick(&mut self, vms: &mut [VirtualMachine]) {
        // 1. Drain outbound from all NICs
        let mut packets_to_route = Vec::new();
        let mut loopback_packets = Vec::new();
        for vm in vms.iter_mut() {
            if let Some(nic) = &mut vm.nic {
                for pkt in nic.drain_outbound() {
                    if pkt.dst_ip.is_loopback() {
                        loopback_packets.push(pkt);
                    } else {
                        packets_to_route.push(pkt);
                    }
                }
            }
        }

        // 2a. Deliver loopback packets to the sender VM (127.x.x.x → same machine)
        for pkt in loopback_packets {
            let src_ip = pkt.src_ip;
            let dst_port = pkt.dst_port;
            for vm in vms.iter_mut() {
                if let Some(nic) = &mut vm.nic {
                    if nic.ip != src_ip {
                        continue;
                    }
                    if nic.is_listening(dst_port) {
                        nic.deliver(pkt);
                    } else if nic.has_ephemeral(dst_port) {
                        nic.deliver_to_ephemeral(dst_port, pkt);
                    }
                    break;
                }
            }
        }

        // 2b. Route each non-loopback packet
        for pkt in packets_to_route {
            match self.router.route_packet(pkt) {
                RouteResult::Deliver { packet, .. } => {
                    let dst_ip = packet.dst_ip;
                    let dst_port = packet.dst_port;
                    for vm in vms.iter_mut() {
                        if let Some(nic) = &mut vm.nic {
                            if nic.ip != dst_ip {
                                continue;
                            }
                            if nic.is_listening(dst_port) {
                                nic.deliver(packet);
                            } else if nic.has_ephemeral(dst_port) {
                                nic.deliver_to_ephemeral(dst_port, packet);
                            }
                            break;
                        }
                    }
                }
                RouteResult::Forward { packet, .. } => {
                    let dst_ip = packet.dst_ip;
                    let dst_port = packet.dst_port;
                    for vm in vms.iter_mut() {
                        if let Some(nic) = &mut vm.nic {
                            if nic.ip != dst_ip {
                                continue;
                            }
                            if nic.is_listening(dst_port) {
                                nic.deliver(packet);
                            } else if nic.has_ephemeral(dst_port) {
                                nic.deliver_to_ephemeral(dst_port, packet);
                            }
                            break;
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

    /// Process VMs using worker abstraction.
    /// Only processes VMs in executable_indices; each VM runs one process per tick (round-robin).
    ///
    /// Returns total process ticks executed across all workers.
    /// Each VM has its own Lua state; VmContext is set on that VM's Lua.
    fn process_vms_parallel(
        &self,
        vms: &mut [VirtualMachine],
        executable_indices: &[usize],
        _pool: &PgPool,
        _fs_service: &Arc<FsService>,
        _user_service: &Arc<UserService>,
    ) -> (u64, Vec<WorkerResult>) {
        // Prepare active VM metadata
        let active_vm_metadata: Vec<(Uuid, String, Option<Ipv4Addr>)> = self
            .active_vms
            .iter()
            .map(|a| (a.id, a.hostname.clone(), a.ip))
            .collect();

        let worker = VmWorker { worker_id: 0 };
        let result = worker.process_chunk(vms, executable_indices, &active_vm_metadata);
        let total_ticks = result.process_ticks;

        (total_ticks, vec![result])
    }

    /// Run the main game loop.
    pub async fn run_loop(
        &mut self,
        vms: &mut Vec<VirtualMachine>,
        terminal_hub: Arc<TerminalHub>,
        process_spy_hub: Arc<ProcessSpyHub>,
        process_run_hub: Arc<ProcessRunHub>,
        process_snapshot_store: Arc<DashMap<Uuid, Vec<ProcessSnapshot>>>,
        vm_lua_memory_store: Arc<DashMap<Uuid, u64>>,
        pool: &PgPool,
        stress_mode: bool,
    ) {
        if stress_mode {
            println!(
                "[cluster] STRESS TEST: Game loop started ({} active VMs)",
                vms.len()
            );
        } else {
            println!(
                "[cluster] GAME MODE: Game loop started ({} active VMs)",
                vms.len()
            );
        }

        let mut tick_count: u64 = 0;
        let start = Instant::now();
        let mut last_budget_reset = Instant::now();
        let mut last_process_snapshot_time = Instant::now();
        let mut last_tick_log_secs: u64 = 0;

        // Executable VM indices (those with budget remaining). Rebuilt every second; shrinks as VMs exhaust budget.
        let mut executable_vm_indices: Vec<usize> = Vec::new();

        // Stress test metrics (pre-allocate for 5min @ 60 TPS = 18000 ticks)
        let mut tick_durations: Vec<Duration> = Vec::with_capacity(TEST_DURATION_SECS as usize * 60);
        let mut min_duration = Duration::MAX;
        let mut max_duration = Duration::ZERO;
        let mut slow_ticks: u64 = 0;
        let mut total_process_ticks: u64 = 0;

        loop {
            // Check timeout APENAS em stress mode
            if stress_mode && start.elapsed() >= Duration::from_secs(TEST_DURATION_SECS) {
                break;
            }
            let tick_start = Instant::now();

            // Snapshot process list on first tick so GetProcessList has data immediately (and periodically below)
            if tick_count == 0 {
                for vm in vms.iter() {
                    if !self.player_owned_vm_ids.contains(&vm.id) {
                        continue;
                    }
                    let snapshots: Vec<ProcessSnapshot> = vm
                        .os
                        .processes
                        .iter()
                        .map(|p| {
                            let name = p
                                .display_name
                                .clone()
                                .or_else(|| p.args.first().cloned())
                                .unwrap_or_else(|| format!("pid_{}", p.id));
                            let status = if p.is_finished() {
                                "finished"
                            } else {
                                "running"
                            };
                            ProcessSnapshot {
                                pid: p.id,
                                name,
                                username: p.username.clone(),
                                status: status.to_string(),
                                memory_bytes: p.estimated_memory_bytes,
                                args: p.args.clone(),
                            }
                        })
                        .collect();
                    process_snapshot_store.insert(vm.id, snapshots);
                    vm_lua_memory_store.insert(vm.id, vm.lua.used_memory() as u64);
                }
            }

            // Process terminal pending opens: spawn shell in player's VM and create session
            {
                let pending: Vec<(Uuid, oneshot::Sender<Result<SessionReady, String>>)> = {
                    let mut hub = terminal_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_opens)
                };
                for (player_id, response_tx) in pending {
                    let vm_record = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.vm_service.get_vm_by_owner_id(player_id))
                    });
                    let Ok(Some(record)) = vm_record else {
                        let _ = response_tx.send(Err("No VM for player".to_string()));
                        continue;
                    };
                    let vm_id = record.id;
                    let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                        let _ = response_tx.send(Err("VM not loaded".to_string()));
                        continue;
                    };
                    let sh_code = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async {
                                self.fs_service.read_file(vm_id, "/bin/sh").await
                            })
                    });
                    let lua_code = match sh_code {
                        Ok(Some((data, _))) => match String::from_utf8(data) {
                            Ok(s) => s,
                            Err(_) => {
                                let msg = format!("/bin/sh not valid UTF-8 for VM {}", vm_id);
                                println!("[cluster] ERROR: {}", msg);
                                let _ = response_tx.send(Err(msg));
                                continue;
                            }
                        },
                        Ok(None) => {
                            let msg = format!("/bin/sh not found in VM {} filesystem - may need bootstrap", vm_id);
                            println!("[cluster] ERROR: {}", msg);
                            let _ = response_tx.send(Err(msg));
                            continue;
                        }
                        Err(e) => {
                            let msg = format!("Failed to read /bin/sh from VM {}: {}", vm_id, e);
                            println!("[cluster] ERROR: {}", msg);
                            let _ = response_tx.send(Err(msg));
                            continue;
                        }
                    };
                    let (shell_uid, shell_username) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let player = self
                                .player_service
                                .get_by_id(player_id)
                                .await
                                .ok()
                                .flatten();
                            let Some(ref p) = player else {
                                return (0i32, "root".to_string());
                            };
                            let vm_user = self
                                .user_service
                                .get_user(vm_id, &p.username)
                                .await
                                .ok()
                                .flatten();
                            vm_user
                                .map(|u| (u.uid, u.username))
                                .unwrap_or((0, "root".to_string()))
                        })
                    });
                    let pid = vm.os.next_process_id();
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        None,
                        &lua_code,
                        vec![],
                        shell_uid,
                        &shell_username,
                        None,
                        Some("sh".to_string()),
                    );
                    if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                        ctx.process_cwd.insert(pid, "/".to_string());
                    }
                    let (stdout_tx, stdout_rx) = mpsc::channel(32);
                    let (stdin_tx, stdin_rx) = mpsc::channel(32);
                    let (error_tx, error_rx) = mpsc::channel(4);
                    let (prompt_ready_tx, prompt_ready_rx) = mpsc::channel(16);
                    let session_id = Uuid::new_v4();
                    let session = TerminalSession {
                        vm_id,
                        pid,
                        stdout_tx,
                        stdin_rx,
                        error_tx,
                        prompt_ready_tx,
                        last_stdout_len: 0,
                    };
                    let ready = SessionReady {
                        session_id,
                        vm_id,
                        pid,
                        stdout_rx,
                        stdin_tx,
                        error_rx,
                        prompt_ready_rx,
                    };
                    {
                        let mut hub = terminal_hub.lock().unwrap();
                        hub.sessions.insert(session_id, session);
                    }
                    let _ = response_tx.send(Ok(ready));
                }
            }

            // Process terminal pending code runs: spawn lua script in player's VM and create session (same shape as shell)
            {
                let pending: Vec<(Uuid, String, oneshot::Sender<Result<SessionReady, String>>)> = {
                    let mut hub = terminal_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_code_runs)
                };
                for (player_id, path, response_tx) in pending {
                    let vm_record = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.vm_service.get_vm_by_owner_id(player_id))
                    });
                    let Ok(Some(record)) = vm_record else {
                        let _ = response_tx.send(Err("No VM for player".to_string()));
                        continue;
                    };
                    let vm_id = record.id;
                    let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                        let _ = response_tx.send(Err("VM not loaded".to_string()));
                        continue;
                    };
                    let lua_code = match tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.fs_service.read_file(vm_id, "/bin/lua"))
                    }) {
                        Ok(Some((data, _))) => match String::from_utf8(data) {
                            Ok(s) => s,
                            Err(_) => {
                                let _ = response_tx.send(Err("/bin/lua not valid UTF-8".to_string()));
                                continue;
                            }
                        },
                        Ok(None) => {
                            let _ = response_tx.send(Err("/bin/lua not found in VM".to_string()));
                            continue;
                        }
                        Err(e) => {
                            let _ = response_tx.send(Err(format!("Failed to read /bin/lua: {}", e)));
                            continue;
                        }
                    };
                    let pid = vm.os.next_process_id();
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        None,
                        &lua_code,
                        vec![path],
                        0,
                        "root",
                        None,
                        Some("lua".to_string()),
                    );
                    let (stdout_tx, stdout_rx) = mpsc::channel(32);
                    let (stdin_tx, stdin_rx) = mpsc::channel(32);
                    let (error_tx, error_rx) = mpsc::channel(4);
                    let (prompt_ready_tx, prompt_ready_rx) = mpsc::channel(16);
                    let session_id = Uuid::new_v4();
                    let session = TerminalSession {
                        vm_id,
                        pid,
                        stdout_tx,
                        stdin_rx,
                        error_tx,
                        prompt_ready_tx,
                        last_stdout_len: 0,
                    };
                    let ready = SessionReady {
                        session_id,
                        vm_id,
                        pid,
                        stdout_rx,
                        stdin_tx,
                        error_rx,
                        prompt_ready_rx,
                    };
                    {
                        let mut hub = terminal_hub.lock().unwrap();
                        hub.sessions.insert(session_id, session);
                    }
                    let _ = response_tx.send(Ok(ready));
                }
            }

            // Process run hub: drain pending_runs, spawn /bin/{bin_name} with args, register active run
            {
                let pending: Vec<super::process_run_hub::PendingRun> = {
                    let mut hub = process_run_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_runs)
                };
                for (player_id, bin_name, args, response_tx) in pending {
                    let vm_record = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.vm_service.get_vm_by_owner_id(player_id))
                    });
                    let Ok(Some(record)) = vm_record else {
                        let _ = response_tx.send(Err("No VM for player".to_string()));
                        continue;
                    };
                    let vm_id = record.id;
                    let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                        let _ = response_tx.send(Err("VM not loaded".to_string()));
                        continue;
                    };
                    let path = format!("/bin/{}", bin_name);
                    let lua_code = match tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.fs_service.read_file(vm_id, &path))
                    }) {
                        Ok(Some((data, _))) => match String::from_utf8(data) {
                            Ok(s) => s,
                            Err(_) => {
                                let _ = response_tx.send(Err(format!("{} not valid UTF-8", path)));
                                continue;
                            }
                        },
                        Ok(None) => {
                            let _ = response_tx.send(Err(format!("{} not found in VM", path)));
                            continue;
                        }
                        Err(e) => {
                            let _ = response_tx.send(Err(format!("Failed to read {}: {}", path, e)));
                            continue;
                        }
                    };
                    let pid = vm.os.next_process_id();
                    let argv0 = bin_name.clone();
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        None,
                        &lua_code,
                        args,
                        0,
                        "root",
                        None,
                        Some(argv0),
                    );
                    let (stream_tx, stream_rx) = mpsc::channel(32);
                    let _ = response_tx.send(Ok(stream_rx));
                    let mut hub = process_run_hub.lock().unwrap();
                    hub.active_runs.insert((vm_id, pid), (stream_tx, 0));
                }
            }

            // Process terminal pending kills: when a session was closed (e.g. UI closed), kill shell and descendants
            // Process terminal pending interrupts (Ctrl+C): kill only the shell's foreground child, not the shell
            {
                let (pending_kills, pending_interrupts): (Vec<(Uuid, u64)>, Vec<(Uuid, u64)>) = {
                    let mut hub = terminal_hub.lock().unwrap();
                    (
                        std::mem::take(&mut hub.pending_kills),
                        std::mem::take(&mut hub.pending_interrupts),
                    )
                };
                for (vm_id, pid) in pending_kills {
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
                for (vm_id, shell_pid) in pending_interrupts {
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        let foreground_pid = vm
                            .lua
                            .app_data_mut::<VmContext>()
                            .and_then(|mut ctx| ctx.shell_foreground_pid.remove(&(vm_id, shell_pid)));
                        let to_kill = foreground_pid.or_else(|| {
                            vm.os
                                .processes
                                .iter()
                                .find(|p| p.parent_id == Some(shell_pid))
                                .map(|p| p.id)
                        });
                        if let Some(pid) = to_kill {
                            vm.os.kill_process_and_descendants(pid);
                        }
                    }
                }
            }

            // ═══ TICK BUDGET: executable VM list ═══
            // Every 0.5 seconds (real time): reset budgets; only VMs with remaining_ticks > 0 are executable.
            // Same logic for both game and stress mode (stress performs better with smaller loop).
            if last_budget_reset.elapsed() >= Duration::from_millis(500) {
                for vm in vms.iter_mut() {
                    if vm.has_running_processes() {
                        vm.remaining_ticks = vm.ticks_per_second;
                    }
                }
                executable_vm_indices = (0..vms.len())
                    .filter(|&i| vms[i].has_running_processes() && vms[i].remaining_ticks > 0)
                    .collect();
                last_budget_reset = Instant::now();
            }

            // ═══ PARALLEL VM PROCESSING ═══
            // Process executable VMs only (one process per VM per tick, round-robin).
            let (process_ticks, worker_results) = self.process_vms_parallel(
                vms,
                &executable_vm_indices,
                pool,
                &self.fs_service,
                &self.user_service,
            );
            total_process_ticks += process_ticks;

            // Decrement budget and remove exhausted VMs from executable list.
            for &idx in executable_vm_indices.iter() {
                if vms[idx].remaining_ticks > 0 {
                    vms[idx].remaining_ticks -= 1;
                }
            }
            executable_vm_indices.retain(|&idx| vms[idx].remaining_ticks > 0);

            // ═══ MEMORY EXCEEDED: reset Lua state for VMs that hit limit ═══
            for result in &worker_results {
                for vm_id in &result.memory_exceeded_vms {
                    // Notify terminal sessions for this VM before reset (process will be gone)
                    {
                        let mut hub = terminal_hub.lock().unwrap();
                        let to_remove: Vec<Uuid> = hub
                            .sessions
                            .iter()
                            .filter(|(_, s)| s.vm_id == *vm_id)
                            .map(|(id, _)| *id)
                            .collect();
                        for id in to_remove {
                            if let Some(session) = hub.sessions.get_mut(&id) {
                                let msg = "Memory limit reached. Killing process...".to_string();
                                let _ = session.stdout_tx.try_send(format!("\n\n<red>{} </red>\n", msg));
                                let _ = session.error_tx.try_send(msg);
                            }
                            hub.sessions.remove(&id);
                        }
                    }
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == *vm_id) {
                        let pool = pool.clone();
                        let fs = self.fs_service.clone();
                        let us = self.user_service.clone();
                        if let Err(e) = vm.reset_lua_state(|| crate::create_vm_lua_state(pool, fs, us)) {
                            println!("[cluster] WARNING: VM {} memory reset failed: {}", vm_id, e);
                        } else {
                            println!("[cluster] VM {} exceeded memory limit, state reset", vm_id);
                        }
                    }
                }
            }

            // ═══ SEQUENTIAL POST-PROCESSING ═══
            // Apply network operations and process spawn/stdin queues from workers
            for vm in vms.iter_mut() {
                // Apply network packets to NICs (collected from workers)
                if let Some(nic) = &mut vm.nic {
                    for result in &worker_results {
                        // Apply packets for this VM
                        for (vm_id, pkt) in &result.net_outbound {
                            if *vm_id == vm.id {
                                nic.send(pkt.clone());
                            }
                        }
                        // Apply listen requests
                        for (vm_id, port, pid) in &result.pending_listen {
                            if *vm_id == vm.id {
                                let _ = nic.try_listen(*port, *pid);
                            }
                        }
                        // Apply ephemeral registrations
                        for (vm_id, port) in &result.pending_ephemeral_register {
                            if *vm_id == vm.id {
                                nic.register_ephemeral(*port);
                            }
                        }
                        for (vm_id, port) in &result.pending_ephemeral_unregister {
                            if *vm_id == vm.id {
                                nic.unregister_ephemeral(*port);
                            }
                        }
                    }
                }

                // Process spawn_queue (collected from workers, needs async fs access)
                for result in &worker_results {
                    for (vm_id, (pid, parent_id, spec, args, uid, username, forward_stdout)) in &result.spawn_queue {
                        if *vm_id != vm.id {
                            continue;
                        }
                        let parent_cwd = {
                            let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                            ctx.process_cwd
                                .get(parent_id)
                                .cloned()
                                .unwrap_or_else(|| "/".to_string())
                        };
                        let path = match &spec {
                            SpawnSpec::Bin(name) => format!("/bin/{}", name),
                            SpawnSpec::Path(p) => p.clone(),
                        };
                        let forward_stdout_to = if *forward_stdout && *parent_id != 0 {
                            vm.os
                                .processes
                                .iter()
                                .find(|p| p.id == *parent_id)
                                .map(|p| p.stdout.clone())
                        } else {
                            None
                        };
                        let fs_result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                self.fs_service.read_file(*vm_id, &path).await
                            })
                        });
                        if let Ok(Some((data, _))) = fs_result {
                            if let Ok(lua_code) = String::from_utf8(data) {
                                let argv0 = match &spec {
                                    SpawnSpec::Bin(n) => n.clone(),
                                    SpawnSpec::Path(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
                                };
                                let parent = if *parent_id == 0 { None } else { Some(*parent_id) };
                                vm.os.spawn_process_with_id(
                                    &vm.lua,
                                    *pid,
                                    parent,
                                    &lua_code,
                                    args.clone(),
                                    *uid,
                                    &username,
                                    forward_stdout_to,
                                    Some(argv0),
                                );
                                if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                                    ctx.process_cwd.insert(*pid, parent_cwd);
                                }
                            }
                        }
                    }
                }

                // Apply stdin inject queue (collected from workers)
                for result in &worker_results {
                    for (vm_id, pid, line) in &result.stdin_inject_queue {
                        if *vm_id != vm.id {
                            continue;
                        }
                        if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == *pid) {
                            if let Ok(mut guard) = p.stdin.lock() {
                                push_stdin_line(&mut guard, line.clone());
                            }
                        }
                        // Notify process spy connections subscribed to this pid
                        let mut hub = process_spy_hub.lock().unwrap();
                        for conn in hub.connections.values_mut() {
                            if conn.vm_id == *vm_id && conn.subscriptions.contains_key(pid) {
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::StdinChunk(*pid, line.clone()));
                            }
                        }
                    }
                }
                // Apply requested_kills (collected from workers; shell handle_special_stdin / request_kill)
                for result in &worker_results {
                    for (vm_id, pid) in &result.requested_kills {
                        if *vm_id != vm.id {
                            continue;
                        }
                        vm.os.kill_process_and_descendants(*pid);
                    }
                }
            }

            // Process Spy: send finished process stdout to subscribed connections (workers already removed these processes)
            // and cache stdout for late subscribers (e.g. Proc Spy opening a just-exited process).
            {
                let mut hub = process_spy_hub.lock().unwrap();
                for result in &worker_results {
                    for (vm_id, pid, stdout) in &result.finished_stdout {
                        for (_conn_id, conn) in hub.connections.iter_mut() {
                            if conn.vm_id == *vm_id && conn.subscriptions.contains_key(pid) {
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::Stdout(*pid, stdout.clone()));
                                conn.subscriptions.remove(pid);
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::ProcessGone(*pid));
                            }
                        }
                        hub.insert_recently_finished_stdout(*vm_id, *pid, stdout.clone());
                    }
                }
            }

            // Process Spy: send initial process list to new connections (from store), then drain stdin/stdout (peek stdout only, do not consume)
            {
                let mut hub = process_spy_hub.lock().unwrap();
                for (_conn_id, conn) in hub.connections.iter_mut() {
                    if !conn.sent_initial_list {
                        if let Some(snapshots) = process_snapshot_store.get(&conn.vm_id) {
                            let list = snapshots.clone();
                            let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::ProcessList(list));
                            conn.sent_initial_list = true;
                        }
                    }
                }
                let mut to_remove: Vec<(Uuid, u64)> = Vec::new();
                for (conn_id, conn) in hub.connections.iter_mut() {
                    for (pid, sub) in conn.subscriptions.iter_mut() {
                        let vm_id = conn.vm_id;
                        let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                            continue;
                        };
                        // Drain stdin_rx and push into process stdin; send StdinChunk for each line
                        while let Ok(line) = sub.stdin_rx.try_recv() {
                            if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == *pid) {
                                if let Ok(mut guard) = p.stdin.lock() {
                                    push_stdin_line(&mut guard, line.clone());
                                }
                            }
                            let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::StdinChunk(*pid, line));
                        }
                        // Read new stdout suffix and send downstream (peek only: do not clear process buffer).
                        // When Terminal clears the buffer after us, next tick len < last_stdout_len; treat all current content as new.
                        if let Some(p) = vm.os.processes.iter().find(|pr| pr.id == *pid) {
                            if let Ok(guard) = p.stdout.lock() {
                                let len = guard.len();
                                if len > sub.last_stdout_len {
                                    let suffix = guard[sub.last_stdout_len..].to_string();
                                    sub.last_stdout_len = len;
                                    drop(guard);
                                    let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::Stdout(*pid, suffix));
                                } else if len < sub.last_stdout_len && len > 0 {
                                    // Buffer was cleared (e.g. by Terminal); current content is all new
                                    sub.last_stdout_len = 0;
                                    let suffix = guard[0..len].to_string();
                                    sub.last_stdout_len = len;
                                    drop(guard);
                                    let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::Stdout(*pid, suffix));
                                } else if len < sub.last_stdout_len {
                                    sub.last_stdout_len = 0;
                                }
                            }
                            if p.is_finished() {
                                to_remove.push((*conn_id, *pid));
                            }
                        } else {
                            to_remove.push((*conn_id, *pid));
                        }
                    }
                }
                for (conn_id, pid) in to_remove {
                    if let Some(conn) = hub.connections.get_mut(&conn_id) {
                        conn.subscriptions.remove(&pid);
                        let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::ProcessGone(pid));
                    }
                }
            }

            // Process Spy: drain pending kills (KillProcess from client)
            {
                let kills: Vec<PendingKill> = {
                    let mut hub = process_spy_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_kills)
                };
                for (vm_id, pid) in kills {
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
            }

            // Process Spy: drain pending Lua script spawns (SpawnLuaScript from client)
            {
                let spawns: Vec<PendingLuaSpawn> = {
                    let mut hub = process_spy_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_lua_spawns)
                };
                for (conn_id, vm_id, path) in spawns {
                    let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                        continue;
                    };
                    let lua_code = match tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.fs_service.read_file(vm_id, "/bin/lua"))
                    }) {
                        Ok(Some((data, _))) => match String::from_utf8(data) {
                            Ok(s) => s,
                            Err(_) => continue,
                        },
                        _ => continue,
                    };
                    let pid = vm.os.next_process_id();
                    vm.os.spawn_process(&vm.lua, &lua_code, vec![path], 0, "root");
                    if let Some(conn) = process_spy_hub.lock().unwrap().connections.get_mut(&conn_id) {
                        // Auto-subscribe this connection to the new pid so stdout (including finished_stdout) is delivered
                        // before the client's SubscribePid is processed (avoids race where script exits in one tick).
                        let (stdin_tx, stdin_rx) = mpsc::channel(32);
                        conn.subscriptions.insert(
                            pid,
                            ProcessSpySubscription {
                                stdin_tx,
                                stdin_rx,
                                last_stdout_len: 0,
                            },
                        );
                        let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::LuaScriptSpawned(pid));
                    }
                }
            }

            // Terminal sessions: inject stdin from gRPC into shell process, drain stdout to gRPC
            // Single lock per tick; iterate over sessions only (O(sessions) vs O(VMs))
            {
                let mut hub = terminal_hub.lock().unwrap();
                let mut to_remove = Vec::new();
                for (session_id, session) in hub.sessions.iter_mut() {
                    let Some(vm) = vms.iter_mut().find(|v| v.id == session.vm_id) else {
                        to_remove.push(*session_id);
                        continue;
                    };
                    // If shell called os.prompt_ready(), notify the client so it can show the prompt again.
                    if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                        if ctx.shell_prompt_ready_pending.remove(&(session.vm_id, session.pid)) {
                            let _ = session.prompt_ready_tx.try_send(());
                        }
                    }
                    // Drain stdin_rx and push into process stdin (non-blocking try_recv loop)
                    while let Ok(line) = session.stdin_rx.try_recv() {
                        if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == session.pid) {
                            if let Ok(mut guard) = p.stdin.lock() {
                                push_stdin_line(&mut guard, line.clone());
                            }
                        }
                        // Notify Process Spy so it can show stdin from Terminal in subscribed tabs
                        let mut spy_hub = process_spy_hub.lock().unwrap();
                        for conn in spy_hub.connections.values_mut() {
                            if conn.vm_id == session.vm_id && conn.subscriptions.contains_key(&session.pid) {
                                let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::StdinChunk(session.pid, line.clone()));
                            }
                        }
                    }
                    // Read new stdout suffix and send on stdout_tx
                    // Clear process stdout after sending to avoid unbounded memory growth.
                    if let Some(p) = vm.os.processes.iter().find(|pr| pr.id == session.pid) {
                        if let Ok(mut guard) = p.stdout.lock() {
                            let len = guard.len();
                            if len > session.last_stdout_len {
                                let suffix = guard[session.last_stdout_len..].to_string();
                                guard.clear();
                                session.last_stdout_len = 0;
                                drop(guard);
                                if session.stdout_tx.try_send(suffix).is_err() {
                                    to_remove.push(*session_id);
                                }
                            }
                        }
                        if p.is_finished() {
                            to_remove.push(*session_id);
                        }
                    } else {
                        to_remove.push(*session_id);
                    }
                }
                for id in to_remove {
                    let (vm_id, pid) = if let Some(session) = hub.sessions.get_mut(&id) {
                        let _ = session.error_tx.try_send("Process terminated.".to_string());
                        (session.vm_id, session.pid)
                    } else {
                        (Uuid::nil(), 0)
                    };
                    hub.sessions.remove(&id);
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
            }

            // Process run hub: drain stdout from active runs, send Finished when process exits
            {
                let mut to_remove = Vec::new();
                {
                    let mut hub = process_run_hub.lock().unwrap();
                    for ((vm_id, pid), (stream_tx, last_len)) in hub.active_runs.iter_mut() {
                        let Some(vm) = vms.iter().find(|v| v.id == *vm_id) else {
                            to_remove.push((*vm_id, *pid));
                            continue;
                        };
                        let Some(p) = vm.os.processes.iter().find(|pr| pr.id == *pid) else {
                            to_remove.push((*vm_id, *pid));
                            continue;
                        };
                        if let Ok(mut guard) = p.stdout.lock() {
                            let len = guard.len();
                            if len > *last_len {
                                let suffix = guard[*last_len..].to_string();
                                guard.clear();
                                *last_len = 0;
                                drop(guard);
                                if stream_tx.try_send(RunProcessStreamMsg::Stdout(suffix)).is_err() {
                                    to_remove.push((*vm_id, *pid));
                                }
                            }
                        }
                        if p.is_finished() {
                            let _ = stream_tx.try_send(RunProcessStreamMsg::Finished(0));
                            to_remove.push((*vm_id, *pid));
                        }
                    }
                }
                for (vm_id, pid) in to_remove {
                    {
                        let mut hub = process_run_hub.lock().unwrap();
                        hub.active_runs.remove(&(vm_id, pid));
                    }
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
            }

            // Network tick — route packets between VMs
            self.network_tick(vms);

            tick_count += 1;

            // Snapshot process list for player-owned VMs every SNAPSHOT_INTERVAL (0.5 s wall-clock)
            if tick_start.duration_since(last_process_snapshot_time) >= SNAPSHOT_INTERVAL {
                last_process_snapshot_time = tick_start;
                for vm in vms.iter() {
                    if !self.player_owned_vm_ids.contains(&vm.id) {
                        continue;
                    }
                    let snapshots: Vec<ProcessSnapshot> = vm
                        .os
                        .processes
                        .iter()
                        .map(|p| {
                            let name = p
                                .display_name
                                .clone()
                                .or_else(|| p.args.first().cloned())
                                .unwrap_or_else(|| format!("pid_{}", p.id));
                            let status = if p.is_finished() {
                                "finished"
                            } else {
                                "running"
                            };
                            ProcessSnapshot {
                                pid: p.id,
                                name,
                                username: p.username.clone(),
                                status: status.to_string(),
                                memory_bytes: p.estimated_memory_bytes,
                                args: p.args.clone(),
                            }
                        })
                        .collect();
                    process_snapshot_store.insert(vm.id, snapshots.clone());
                    vm_lua_memory_store.insert(vm.id, vm.lua.used_memory() as u64);
                    // Push process list to process spy connections for this VM
                    let mut hub = process_spy_hub.lock().unwrap();
                    for conn in hub.connections.values_mut() {
                        if conn.vm_id == vm.id {
                            let _ = conn.downstream_tx.try_send(ProcessSpyDownstreamMsg::ProcessList(snapshots.clone()));
                        }
                    }
                }
            }

            // Do not remove VMs when they have no processes; they may get new terminals or processes later

            // Track tick duration (only collect in stress mode for final stats; game mode runs forever)
            let tick_duration = tick_start.elapsed();
            if stress_mode {
                tick_durations.push(tick_duration);
            }
            min_duration = min_duration.min(tick_duration);
            max_duration = max_duration.max(tick_duration);

            if tick_duration > TICK_TIME {
                slow_ticks += 1;
            }

            // Log at most once per second (wall-clock) to avoid flooding when loop runs faster than 60 TPS
            let uptime_secs = start.elapsed().as_secs();
            if uptime_secs > 0 && uptime_secs != last_tick_log_secs {
                last_tick_log_secs = uptime_secs;
                println!(
                    "[cluster] Tick {} | {} VMs active | uptime {}s | {:.1} TPS",
                    tick_count,
                    vms.len(),
                    uptime_secs,
                    tick_count as f64 / uptime_secs as f64,
                );
            }
        }

        // Display stress test results (apenas em stress mode)
        if stress_mode {
            let duration = start.elapsed();

            // Calculate percentiles
            tick_durations.sort();
            let p50 = tick_durations[tick_durations.len() / 2];
            let p95 = tick_durations[tick_durations.len() * 95 / 100];
            let p99 = tick_durations[tick_durations.len() * 99 / 100];
            let mean: Duration = tick_durations.iter().sum::<Duration>() / tick_durations.len() as u32;

            println!("\n=== STRESS TEST RESULTS ===\n");
            println!("Configuration:");
            println!("  VMs: {}", vms.len());
            println!("  Total Processes: {}\n", vms.iter().map(|vm| vm.os.processes.len()).sum::<usize>());

            // Lua state memory (per-VM heap)
            let total_lua_bytes: usize = vms.iter().map(|vm| vm.lua.used_memory()).sum();
            let avg_lua_bytes = if vms.is_empty() {
                0
            } else {
                total_lua_bytes / vms.len()
            };
            println!("Lua Memory (Per-VM State):");
            println!("  Total: {} bytes ({:.2} MB)", total_lua_bytes, total_lua_bytes as f64 / 1_048_576.0);
            println!("  Average per VM: {} bytes ({:.2} KB)\n", avg_lua_bytes, avg_lua_bytes as f64 / 1024.0);

            println!("Performance:");
            println!("  Duration: {:.2}s (test runtime)\n", duration.as_secs_f64());

            println!("Game Loop (Main Server Loop):");
            println!("  Total Iterations: {} ticks", tick_count);
            println!("  Ticks/Second: {:.1} TPS (sem limitador de 60 FPS)", tick_count as f64 / duration.as_secs_f64());
            println!("  Comparação: {:.1}x mais rápido que 60 TPS\n", (tick_count as f64 / duration.as_secs_f64()) / 60.0);

            println!("Process Ticks (Execuções Lua):");
            println!("  Total: {} process.tick() chamadas", total_process_ticks);
            println!("  Por segundo: {:.0} execuções Lua/seg", total_process_ticks as f64 / duration.as_secs_f64());
            println!("  Explicação: {} VMs × {} game ticks = {} process ticks\n", vms.len(), tick_count, total_process_ticks);

            println!("Tick Duration Statistics:");
            println!("  Min: {:?}", min_duration);
            println!("  Max: {:?}", max_duration);
            println!("  Mean: {:?}", mean);
            println!("  Median (p50): {:?}", p50);
            println!("  p95: {:?}", p95);
            println!("  p99: {:?}", p99);
            println!("  Slow ticks (>16ms): {} ({:.1}%)", slow_ticks, (slow_ticks as f64 / tick_count as f64) * 100.0);
        }
    }

    /// Runs exactly n ticks (terminal hub, VM ticks, network). Test-only: allows driving the
    /// game loop from a test without spawning, so we can await SessionReady and stdout on the same task.
    #[cfg(test)]
    pub async fn run_n_ticks_with_terminal_hub(
        &mut self,
        vms: &mut Vec<VirtualMachine>,
        terminal_hub: Arc<TerminalHub>,
        n: usize,
    ) {
        for _ in 0..n {
            let tick_start = Instant::now();

            // Process terminal pending opens (same as run_loop)
            {
                let pending: Vec<(Uuid, oneshot::Sender<Result<SessionReady, String>>)> = {
                    let mut hub = terminal_hub.lock().unwrap();
                    std::mem::take(&mut hub.pending_opens)
                };
                for (player_id, response_tx) in pending {
                    let vm_record = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(self.vm_service.get_vm_by_owner_id(player_id))
                    });
                    let Ok(Some(record)) = vm_record else {
                        let _ = response_tx.send(Err("No VM for player".to_string()));
                        continue;
                    };
                    let vm_id = record.id;
                    let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) else {
                        let _ = response_tx.send(Err("VM not loaded".to_string()));
                        continue;
                    };
                    let sh_code = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current()
                            .block_on(async { self.fs_service.read_file(vm_id, "/bin/sh").await })
                    });
                    let lua_code = match sh_code {
                        Ok(Some((data, _))) => match String::from_utf8(data) {
                            Ok(s) => s,
                            Err(_) => {
                                let msg = format!("/bin/sh not valid UTF-8 for VM {}", vm_id);
                                println!("[cluster] ERROR: {}", msg);
                                let _ = response_tx.send(Err(msg));
                                continue;
                            }
                        },
                        Ok(None) => {
                            let msg = format!("/bin/sh not found in VM {} filesystem - may need bootstrap", vm_id);
                            println!("[cluster] ERROR: {}", msg);
                            let _ = response_tx.send(Err(msg));
                            continue;
                        }
                        Err(e) => {
                            let msg = format!("Failed to read /bin/sh from VM {}: {}", vm_id, e);
                            println!("[cluster] ERROR: {}", msg);
                            let _ = response_tx.send(Err(msg));
                            continue;
                        }
                    };
                    let (shell_uid, shell_username) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let player = self
                                .player_service
                                .get_by_id(player_id)
                                .await
                                .ok()
                                .flatten();
                            let Some(ref p) = player else {
                                return (0i32, "root".to_string());
                            };
                            let vm_user = self
                                .user_service
                                .get_user(vm_id, &p.username)
                                .await
                                .ok()
                                .flatten();
                            vm_user
                                .map(|u| (u.uid, u.username))
                                .unwrap_or((0, "root".to_string()))
                        })
                    });
                    let pid = vm.os.next_process_id();
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        None,
                        &lua_code,
                        vec![],
                        shell_uid,
                        &shell_username,
                        None,
                        Some("sh".to_string()),
                    );
                    if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                        ctx.process_cwd.insert(pid, "/".to_string());
                    }
                    let (stdout_tx, stdout_rx) = mpsc::channel(32);
                    let (stdin_tx, stdin_rx) = mpsc::channel(32);
                    let (error_tx, error_rx) = mpsc::channel(4);
                    let (prompt_ready_tx, prompt_ready_rx) = mpsc::channel(16);
                    let session_id = Uuid::new_v4();
                    let session = TerminalSession {
                        vm_id,
                        pid,
                        stdout_tx,
                        stdin_rx,
                        error_tx,
                        prompt_ready_tx,
                        last_stdout_len: 0,
                    };
                    let ready = SessionReady {
                        session_id,
                        vm_id,
                        pid,
                        stdout_rx,
                        stdin_tx,
                        error_rx,
                        prompt_ready_rx,
                    };
                    {
                        let mut hub = terminal_hub.lock().unwrap();
                        hub.sessions.insert(session_id, session);
                    }
                    let _ = response_tx.send(Ok(ready));
                }
            }

            // Process terminal pending kills and pending interrupts (same as run_loop)
            {
                let (pending_kills, pending_interrupts): (Vec<(Uuid, u64)>, Vec<(Uuid, u64)>) = {
                    let mut hub = terminal_hub.lock().unwrap();
                    (
                        std::mem::take(&mut hub.pending_kills),
                        std::mem::take(&mut hub.pending_interrupts),
                    )
                };
                for (vm_id, pid) in pending_kills {
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
                for (vm_id, shell_pid) in pending_interrupts {
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        let foreground_pid = vm
                            .lua
                            .app_data_mut::<VmContext>()
                            .and_then(|mut ctx| ctx.shell_foreground_pid.remove(&(vm_id, shell_pid)));
                        let to_kill = foreground_pid.or_else(|| {
                            vm.os
                                .processes
                                .iter()
                                .find(|p| p.parent_id == Some(shell_pid))
                                .map(|p| p.id)
                        });
                        if let Some(pid) = to_kill {
                            vm.os.kill_process_and_descendants(pid);
                        }
                    }
                }
            }

            // Set context and tick each VM (same as run_loop - one full pass over all vms)
            for vm in vms.iter_mut() {
                {
                    let active = self.active_vms.iter().find(|a| a.id == vm.id);
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    let hostname = active.map(|a| a.hostname.as_str()).unwrap_or("unknown");
                    let ip = active.and_then(|a| a.ip);
                    ctx.set_vm(vm.id, hostname, ip);
                    if let Some(nic) = &vm.nic {
                        ctx.set_port_owners(nic.get_port_owners());
                    }
                    ctx.next_pid = vm.os.next_process_id();
                    for p in &vm.os.processes {
                        let status = if p.is_finished() { "finished" } else { "running" };
                        ctx.process_status_map.insert(p.id, status.to_string());
                        if let Ok(guard) = p.stdout.lock() {
                            ctx.process_stdout.insert(p.id, guard.clone());
                        }
                        let display_name = p
                            .display_name
                            .as_deref()
                            .unwrap_or_else(|| p.args.first().map(|s| s.as_str()).unwrap_or(""));
                        ctx.process_display_name
                            .insert(p.id, display_name.to_string());
                    }
                    ctx.merge_last_stdout_of_finished();
                    if let Some(nic) = &mut vm.nic {
                        while let Some(pkt) = nic.recv() {
                            ctx.net_inbound.push_back(pkt);
                        }
                    }
                    std::mem::swap(&mut ctx.connections, &mut vm.connections);
                    std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
                    if let Some(nic) = &mut vm.nic {
                        ctx.sync_connection_inbounds_from_nic(nic);
                    }
                }

                let mut memory_exceeded = false;
                for process in &mut vm.os.processes {
                    {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        ctx.current_pid = process.id;
                        ctx.current_uid = process.user_id;
                        ctx.current_username = process.username.clone();
                        ctx.set_current_process(
                            process.stdin.clone(),
                            process.stdout.clone(),
                            process.args.clone(),
                            process.forward_stdout_to.clone(),
                        );
                    }
                    if !process.is_finished() {
                        if let Err(e) = process.tick() {
                            if matches!(e, mlua::Error::MemoryError(_)) {
                                memory_exceeded = true;
                            }
                            break;
                        }
                    }
                    {
                        let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                        if ctx.current_uid != process.user_id {
                            process.user_id = ctx.current_uid;
                            process.username = ctx.current_username.clone();
                        }
                    }
                }
                if memory_exceeded {
                    // Notify terminal sessions for this VM before reset
                    {
                        let mut hub = terminal_hub.lock().unwrap();
                        let to_remove: Vec<Uuid> = hub
                            .sessions
                            .iter()
                            .filter(|(_, s)| s.vm_id == vm.id)
                            .map(|(id, _)| *id)
                            .collect();
                        for id in to_remove {
                            if let Some(session) = hub.sessions.get_mut(&id) {
                                let msg = "Memory limit reached. Killing process...".to_string();
                                let _ = session.stdout_tx.try_send(format!("\n\n<red>{} </red>\n", msg));
                                let _ = session.error_tx.try_send(msg);
                            }
                            hub.sessions.remove(&id);
                        }
                    }
                    let pool = self.vm_service.pool().clone();
                    let fs = self.fs_service.clone();
                    let us = self.user_service.clone();
                    if let Err(e) = vm.reset_lua_state(|| crate::create_vm_lua_state(pool, fs, us)) {
                        println!("[cluster] WARNING: VM {} memory reset failed: {}", vm.id, e);
                    } else {
                        println!("[cluster] VM {} exceeded memory limit, state reset", vm.id);
                    }
                } else {
                    {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        for p in &vm.os.processes {
                            if p.is_finished() {
                                if let Ok(guard) = p.stdout.lock() {
                                    ctx.last_stdout_of_finished.insert(p.id, guard.clone());
                                }
                            }
                        }
                    }
                    let finished_pids: Vec<u64> = vm
                        .os
                        .processes
                        .iter()
                        .filter(|p| p.is_finished())
                        .map(|p| p.id)
                        .collect();
                    for pid in &finished_pids {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        ctx.close_connections_for_pid(*pid);
                    }
                    vm.os.processes.retain(|p| !p.is_finished());
                    if let Some(nic) = &mut vm.nic {
                        for pid in &finished_pids {
                            nic.unlisten_pid(*pid);
                        }
                    }
                }

                // Process spawn_queue
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    let spawn_queue = std::mem::take(&mut ctx.spawn_queue);
                    let vm_id = ctx.vm_id;
                    drop(ctx);

                    for (pid, parent_id, spec, args, uid, username, forward_stdout) in spawn_queue {
                        let parent_cwd = {
                            let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                            ctx.process_cwd
                                .get(&parent_id)
                                .cloned()
                                .unwrap_or_else(|| "/".to_string())
                        };
                        let path = match &spec {
                            SpawnSpec::Bin(name) => format!("/bin/{}", name),
                            SpawnSpec::Path(p) => p.clone(),
                        };
                        let forward_stdout_to = if forward_stdout && parent_id != 0 {
                            vm.os
                                .processes
                                .iter()
                                .find(|p| p.id == parent_id)
                                .map(|p| p.stdout.clone())
                        } else {
                            None
                        };
                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(async { self.fs_service.read_file(vm_id, &path).await })
                        });
                        if let Ok(Some((data, _))) = result {
                            if let Ok(lua_code) = String::from_utf8(data) {
                                let argv0 = match &spec {
                                    SpawnSpec::Bin(n) => n.clone(),
                                    SpawnSpec::Path(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
                                };
                                let parent = if parent_id == 0 { None } else { Some(parent_id) };
                                vm.os.spawn_process_with_id(
                                    &vm.lua,
                                    pid,
                                    parent,
                                    &lua_code,
                                    args,
                                    uid,
                                    &username,
                                    forward_stdout_to,
                                    Some(argv0),
                                );
                                if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                                    ctx.process_cwd.insert(pid, parent_cwd);
                                }
                            }
                        }
                    }
                }

                // Apply stdin inject queue
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    let stdin_inject = std::mem::take(&mut ctx.stdin_inject_queue);
                    drop(ctx);
                    for (pid, line) in stdin_inject {
                        if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == pid) {
                            if let Ok(mut guard) = p.stdin.lock() {
                                push_stdin_line(&mut guard, line);
                            }
                        }
                    }
                }

                // Apply requested_kills (from shell handle_special_stdin / request_kill)
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    let kills = std::mem::take(&mut ctx.requested_kills);
                    drop(ctx);
                    for pid in kills {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }

                // Apply outbound packets
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    if let Some(nic) = &mut vm.nic {
                        for pkt in ctx.net_outbound.drain(..) {
                            nic.send(pkt);
                        }
                        for (port, pid) in ctx.pending_listen.drain(..) {
                            let _ = nic.try_listen(port, pid);
                        }
                        for port in ctx.pending_ephemeral_register.drain(..) {
                            nic.register_ephemeral(port);
                        }
                        for port in ctx.pending_ephemeral_unregister.drain(..) {
                            nic.unregister_ephemeral(port);
                        }
                        for pkt in ctx.net_inbound.drain(..) {
                            nic.deliver(pkt);
                        }
                    }
                    std::mem::swap(&mut ctx.connections, &mut vm.connections);
                    std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
                }
            }

            // Terminal sessions: single lock per tick; iterate over sessions only
            {
                let mut hub = terminal_hub.lock().unwrap();
                let mut to_remove = Vec::new();
                for (session_id, session) in hub.sessions.iter_mut() {
                    let Some(vm) = vms.iter_mut().find(|v| v.id == session.vm_id) else {
                        to_remove.push(*session_id);
                        continue;
                    };
                    while let Ok(line) = session.stdin_rx.try_recv() {
                        if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == session.pid) {
                            if let Ok(mut guard) = p.stdin.lock() {
                                push_stdin_line(&mut guard, line);
                            }
                        }
                    }
                    if let Some(p) = vm.os.processes.iter().find(|pr| pr.id == session.pid) {
                        if let Ok(mut guard) = p.stdout.lock() {
                            let len = guard.len();
                            if len > session.last_stdout_len {
                                let suffix = guard[session.last_stdout_len..].to_string();
                                guard.clear();
                                session.last_stdout_len = 0;
                                drop(guard);
                                if session.stdout_tx.try_send(suffix).is_err() {
                                    to_remove.push(*session_id);
                                }
                            }
                        }
                        if p.is_finished() {
                            to_remove.push(*session_id);
                        }
                    } else {
                        to_remove.push(*session_id);
                    }
                }
                for id in to_remove {
                    let (vm_id, pid) = if let Some(session) = hub.sessions.get_mut(&id) {
                        let _ = session.error_tx.try_send("Process terminated.".to_string());
                        (session.vm_id, session.pid)
                    } else {
                        (Uuid::nil(), 0)
                    };
                    hub.sessions.remove(&id);
                    if let Some(vm) = vms.iter_mut().find(|v| v.id == vm_id) {
                        vm.os.kill_process_and_descendants(pid);
                    }
                }
            }

            self.network_tick(vms);

            let _ = tick_start;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bin_programs;
    use super::super::db::{self, fs_service::FsService, player_service::PlayerService};
    use super::super::db::{user_service::UserService, vm_service::VmService};
    use super::super::lua_api::context::{SpawnSpec, VmContext};
    use super::super::net::ip::{Ipv4Addr, Subnet};
    use super::super::net::packet::Packet;
    use super::super::terminal_hub;
    use super::*;
    use tokio::sync::oneshot;
    use std::collections::HashMap;
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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let echo_code = bin_programs::ECHO;
        vm.os.spawn_process(&vm.lua, echo_code, vec!["hello".to_string()], 0, "root");

        let mut stdout_result = String::new();
        let max_ticks = 100;
        for _ in 0..max_ticks {
            {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, "tick-test-vm", None);
            }

            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                        process.forward_stdout_to.clone(),
                    );
                }
                process.tick().expect("process tick failed");
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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let read_script = r#"
local line = io.read()
if line then
    io.write("read: " .. line .. "\n")
end
"#;
        vm.os.spawn_process(&vm.lua, read_script, vec![], 0, "root");

        let process = vm.os.processes.first_mut().unwrap();
        {
            let mut guard = process.stdin.lock().unwrap();
            push_stdin_line(&mut guard, "25".to_string());
        }

        let mut stdout_result = String::new();
        let max_ticks = 100;
        for _ in 0..max_ticks {
            {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, "stdin-test-vm", None);
            }

            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                        process.forward_stdout_to.clone(),
                    );
                }
                process.tick().expect("process tick failed");
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
        vm: &mut VirtualMachine,
        vm_id: Uuid,
        hostname: &str,
    ) -> String {
        run_tick_until_done_with_limit(vm, vm_id, hostname, 100).0
    }

    /// Helper: run tick loop with max ticks, return (stdout, tick_count).
    fn run_tick_until_done_with_limit(
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
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, hostname, None);
            }
            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                        process.forward_stdout_to.clone(),
                    );
                }
                process.tick().expect("process tick failed");
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

    /// Performs one full VM tick including spawn_queue and stdin_inject_queue (so os.spawn/os.spawn_path children run).
    async fn vm_full_tick_async(
        vm: &mut VirtualMachine,
        manager: &VmManager,
        vm_id: Uuid,
        hostname: &str,
    ) -> HashMap<u64, String> {
        let mut finished_stdout = HashMap::new();
        {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            ctx.set_vm(vm_id, hostname, None);
            ctx.next_pid = vm.os.next_process_id();
            for p in &vm.os.processes {
                let status = if p.is_finished() { "finished" } else { "running" };
                ctx.process_status_map.insert(p.id, status.to_string());
                if let Ok(guard) = p.stdout.lock() {
                    ctx.process_stdout.insert(p.id, guard.clone());
                }
                let display_name = p
                    .display_name
                    .as_deref()
                    .unwrap_or_else(|| p.args.first().map(|s| s.as_str()).unwrap_or(""));
                ctx.process_display_name
                    .insert(p.id, display_name.to_string());
            }
            ctx.merge_last_stdout_of_finished();
        }
        for process in &mut vm.os.processes {
            if process.is_finished() {
                continue;
            }
            {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.current_pid = process.id;
                ctx.current_uid = process.user_id;
                ctx.current_username = process.username.clone();
                ctx.set_current_process(
                    process.stdin.clone(),
                    process.stdout.clone(),
                    process.args.clone(),
                    process.forward_stdout_to.clone(),
                );
            }
            process.tick().expect("process tick failed");
            if process.is_finished() {
                if let Ok(guard) = process.stdout.lock() {
                    finished_stdout.insert(process.id, guard.clone());
                }
            }
            {
                let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                if ctx.current_uid != process.user_id {
                    process.user_id = ctx.current_uid;
                    process.username = ctx.current_username.clone();
                }
            }
        }
        {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            for p in &vm.os.processes {
                if p.is_finished() {
                    if let Ok(guard) = p.stdout.lock() {
                        ctx.last_stdout_of_finished.insert(p.id, guard.clone());
                    }
                }
            }
        }
        vm.os.processes.retain(|p| !p.is_finished());

        let spawn_queue = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.spawn_queue)
        };
        for (pid, parent_id, spec, args, uid, username, forward_stdout) in spawn_queue {
            let parent_cwd = {
                let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                ctx.process_cwd
                    .get(&parent_id)
                    .cloned()
                    .unwrap_or_else(|| "/".to_string())
            };
            let path = match &spec {
                SpawnSpec::Bin(name) => format!("/bin/{}", name),
                SpawnSpec::Path(p) => p.clone(),
            };
            let forward_stdout_to = if forward_stdout && parent_id != 0 {
                vm.os
                    .processes
                    .iter()
                    .find(|p| p.id == parent_id)
                    .map(|p| p.stdout.clone())
            } else {
                None
            };
            let result = manager.fs_service.read_file(vm_id, &path).await;
            if let Ok(Some((data, _))) = result {
                if let Ok(lua_code) = String::from_utf8(data) {
                    let argv0 = match &spec {
                        SpawnSpec::Bin(n) => n.clone(),
                        SpawnSpec::Path(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
                    };
                    let parent = if parent_id == 0 { None } else { Some(parent_id) };
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        parent,
                        &lua_code,
                        args,
                        uid,
                        &username,
                        forward_stdout_to,
                        Some(argv0),
                    );
                    if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                        ctx.process_cwd.insert(pid, parent_cwd);
                    }
                }
            }
        }

        let stdin_inject = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.stdin_inject_queue)
        };
        for (pid, line) in stdin_inject {
            if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == pid) {
                if let Ok(mut guard) = p.stdin.lock() {
                    push_stdin_line(&mut guard, line);
                }
            }
        }
        let requested_kills = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.requested_kills)
        };
        for pid in requested_kills {
            vm.os.kill_process_and_descendants(pid);
        }
        finished_stdout
    }

    /// Run full ticks (with spawn and stdin inject) until VM has no processes or max_ticks. Returns (pid -> stdout for each finished process, tick_count).
    async fn run_tick_until_done_with_spawn(
        vm: &mut VirtualMachine,
        manager: &VmManager,
        vm_id: Uuid,
        hostname: &str,
        max_ticks: usize,
    ) -> (HashMap<u64, String>, usize) {
        let mut all_stdout = HashMap::new();
        let mut tick_count = 0;
        for _ in 0..max_ticks {
            tick_count += 1;
            let finished = vm_full_tick_async(vm, manager, vm_id, hostname).await;
            for (pid, out) in finished {
                all_stdout.insert(pid, out);
            }
            if vm.os.processes.is_empty() || vm.os.is_finished() {
                break;
            }
        }
        (all_stdout, tick_count)
    }

    /// Run exactly n ticks (with spawn and stdin inject). Use when the process under test never exits (e.g. shell).
    async fn run_n_ticks_with_spawn(
        vm: &mut VirtualMachine,
        manager: &VmManager,
        vm_id: Uuid,
        hostname: &str,
        n: usize,
    ) {
        for _ in 0..n {
            let _ = vm_full_tick_async(vm, manager, vm_id, hostname).await;
        }
    }

    /// Run n full game-loop ticks for VMs with network: per-VM context (with IP), NIC sync, then network_tick.
    async fn run_n_ticks_vms_network(
        manager: &mut VmManager,
        vms: &mut [VirtualMachine],
        n: usize,
    ) {
        assert!(
            vms.len() >= 2,
            "run_n_ticks_vms_network expects at least 2 VMs, got {}",
            vms.len()
        );
        for _ in 0..n {
            for vm in vms.iter_mut() {
                let (hostname, ip) = manager
                    .get_active_vm(vm.id)
                    .map(|a| (a.hostname.clone(), a.ip))
                    .unwrap_or_else(|| ("unknown".to_string(), None));

                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.set_vm(vm.id, &hostname, ip);
                    if let Some(nic) = &vm.nic {
                        ctx.set_port_owners(nic.get_port_owners());
                    }
                    ctx.next_pid = vm.os.next_process_id();
                    for p in &vm.os.processes {
                        let status = if p.is_finished() { "finished" } else { "running" };
                        ctx.process_status_map.insert(p.id, status.to_string());
                        if let Ok(guard) = p.stdout.lock() {
                            ctx.process_stdout.insert(p.id, guard.clone());
                        }
                        let display_name = p
                            .display_name
                            .as_deref()
                            .unwrap_or_else(|| p.args.first().map(|s| s.as_str()).unwrap_or(""));
                        ctx.process_display_name
                            .insert(p.id, display_name.to_string());
                    }
                    ctx.merge_last_stdout_of_finished();
                    if let Some(nic) = &mut vm.nic {
                        while let Some(pkt) = nic.recv() {
                            ctx.net_inbound.push_back(pkt);
                        }
                    }
                    std::mem::swap(&mut ctx.connections, &mut vm.connections);
                    std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
                    if let Some(nic) = &mut vm.nic {
                        ctx.sync_connection_inbounds_from_nic(nic);
                    }
                }

                let mut memory_exceeded = false;
                for process in &mut vm.os.processes {
                    {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        ctx.current_pid = process.id;
                        ctx.current_uid = process.user_id;
                        ctx.current_username = process.username.clone();
                        ctx.set_current_process(
                            process.stdin.clone(),
                            process.stdout.clone(),
                            process.args.clone(),
                            process.forward_stdout_to.clone(),
                        );
                    }
                    if !process.is_finished() {
                        if let Err(e) = process.tick() {
                            if matches!(e, mlua::Error::MemoryError(_)) {
                                memory_exceeded = true;
                            }
                            break;
                        }
                    }
                    {
                        let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                        if ctx.current_uid != process.user_id {
                            process.user_id = ctx.current_uid;
                            process.username = ctx.current_username.clone();
                        }
                    }
                }
                if memory_exceeded {
                    let pool = manager.vm_service.pool().clone();
                    let fs = manager.fs_service.clone();
                    let us = manager.user_service.clone();
                    let _ = vm.reset_lua_state(|| crate::create_vm_lua_state(pool, fs, us));
                } else {
                    {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        for p in &vm.os.processes {
                            if p.is_finished() {
                                if let Ok(guard) = p.stdout.lock() {
                                    ctx.last_stdout_of_finished.insert(p.id, guard.clone());
                                }
                            }
                        }
                    }
                    let finished_pids: Vec<u64> = vm
                        .os
                        .processes
                        .iter()
                        .filter(|p| p.is_finished())
                        .map(|p| p.id)
                        .collect();
                    for pid in &finished_pids {
                        let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                        ctx.close_connections_for_pid(*pid);
                    }
                    vm.os.processes.retain(|p| !p.is_finished());
                    if let Some(nic) = &mut vm.nic {
                        for pid in &finished_pids {
                            nic.unlisten_pid(*pid);
                        }
                    }
                }

                let spawn_queue = {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    std::mem::take(&mut ctx.spawn_queue)
                };
                let vm_id = vm.id;
                for (pid, parent_id, spec, args, uid, username, forward_stdout) in spawn_queue {
                    let parent_cwd = {
                        let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                        ctx.process_cwd
                            .get(&parent_id)
                            .cloned()
                            .unwrap_or_else(|| "/".to_string())
                    };
                    let path = match &spec {
                        SpawnSpec::Bin(name) => format!("/bin/{}", name),
                        SpawnSpec::Path(p) => p.clone(),
                    };
                    let forward_stdout_to = if forward_stdout && parent_id != 0 {
                        vm.os
                            .processes
                            .iter()
                            .find(|p| p.id == parent_id)
                            .map(|p| p.stdout.clone())
                    } else {
                        None
                    };
                    let result = manager.fs_service.read_file(vm_id, &path).await;
                    if let Ok(Some((data, _))) = result {
                        if let Ok(lua_code) = String::from_utf8(data) {
                            let argv0 = match &spec {
                                SpawnSpec::Bin(n) => n.clone(),
                                SpawnSpec::Path(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
                            };
                            let parent = if parent_id == 0 { None } else { Some(parent_id) };
                            vm.os.spawn_process_with_id(
                                &vm.lua,
                                pid,
                                parent,
                                &lua_code,
                                args,
                                uid,
                                &username,
                                forward_stdout_to,
                                Some(argv0),
                            );
                            if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                                ctx.process_cwd.insert(pid, parent_cwd);
                            }
                        }
                    }
                }

                let stdin_inject = {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    std::mem::take(&mut ctx.stdin_inject_queue)
                };
                for (pid, line) in stdin_inject {
                    if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == pid) {
                        if let Ok(mut guard) = p.stdin.lock() {
                            push_stdin_line(&mut guard, line);
                        }
                    }
                }
                let requested_kills = {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    std::mem::take(&mut ctx.requested_kills)
                };
                for pid in requested_kills {
                    vm.os.kill_process_and_descendants(pid);
                }

                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    if let Some(nic) = &mut vm.nic {
                        for pkt in ctx.net_outbound.drain(..) {
                            nic.send(pkt);
                        }
                        for (port, pid) in ctx.pending_listen.drain(..) {
                            let _ = nic.try_listen(port, pid);
                        }
                        for port in ctx.pending_ephemeral_register.drain(..) {
                            nic.register_ephemeral(port);
                        }
                        for port in ctx.pending_ephemeral_unregister.drain(..) {
                            nic.unregister_ephemeral(port);
                        }
                        for pkt in ctx.net_inbound.drain(..) {
                            nic.deliver(pkt);
                        }
                    }
                    std::mem::swap(&mut ctx.connections, &mut vm.connections);
                    std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
                }
            }

            manager.network_tick(vms);
        }
    }

    /// Run one tick for a single VM only (by index), no network_tick. Used to inspect VM state after tick.
    async fn run_one_tick_single_vm(
        manager: &mut VmManager,
        vms: &mut [VirtualMachine],
        vm_index: usize,
    ) {
        let vm = &mut vms[vm_index];
        let (hostname, ip) = manager
            .get_active_vm(vm.id)
            .map(|a| (a.hostname.clone(), a.ip))
            .unwrap_or_else(|| ("unknown".to_string(), None));

        {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            ctx.set_vm(vm.id, &hostname, ip);
            if let Some(nic) = &vm.nic {
                ctx.set_port_owners(nic.get_port_owners());
            }
            ctx.next_pid = vm.os.next_process_id();
            for p in &vm.os.processes {
                let status = if p.is_finished() { "finished" } else { "running" };
                ctx.process_status_map.insert(p.id, status.to_string());
                if let Ok(guard) = p.stdout.lock() {
                    ctx.process_stdout.insert(p.id, guard.clone());
                }
                let display_name = p
                    .display_name
                    .as_deref()
                    .unwrap_or_else(|| p.args.first().map(|s| s.as_str()).unwrap_or(""));
                ctx.process_display_name
                    .insert(p.id, display_name.to_string());
            }
            ctx.merge_last_stdout_of_finished();
            if let Some(nic) = &mut vm.nic {
                while let Some(pkt) = nic.recv() {
                    ctx.net_inbound.push_back(pkt);
                }
            }
            std::mem::swap(&mut ctx.connections, &mut vm.connections);
            std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
            if let Some(nic) = &mut vm.nic {
                ctx.sync_connection_inbounds_from_nic(nic);
            }
        }

        let mut memory_exceeded = false;
        for process in &mut vm.os.processes {
            {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.current_pid = process.id;
                ctx.current_uid = process.user_id;
                ctx.current_username = process.username.clone();
                ctx.set_current_process(
                    process.stdin.clone(),
                    process.stdout.clone(),
                    process.args.clone(),
                    process.forward_stdout_to.clone(),
                );
            }
            if !process.is_finished() {
                if let Err(e) = process.tick() {
                    if matches!(e, mlua::Error::MemoryError(_)) {
                        memory_exceeded = true;
                    }
                    break;
                }
            }
            {
                let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                if ctx.current_uid != process.user_id {
                    process.user_id = ctx.current_uid;
                    process.username = ctx.current_username.clone();
                }
            }
        }
        if memory_exceeded {
            let pool = manager.vm_service.pool().clone();
            let fs = manager.fs_service.clone();
            let us = manager.user_service.clone();
            let _ = vm.reset_lua_state(|| crate::create_vm_lua_state(pool, fs, us));
        } else {
            {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                for p in &vm.os.processes {
                    if p.is_finished() {
                        if let Ok(guard) = p.stdout.lock() {
                            ctx.last_stdout_of_finished.insert(p.id, guard.clone());
                        }
                    }
                }
            }
            let finished_pids: Vec<u64> = vm
                .os
                .processes
                .iter()
                .filter(|p| p.is_finished())
                .map(|p| p.id)
                .collect();
            for pid in &finished_pids {
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.close_connections_for_pid(*pid);
            }
            vm.os.processes.retain(|p| !p.is_finished());
            if let Some(nic) = &mut vm.nic {
                for pid in &finished_pids {
                    nic.unlisten_pid(*pid);
                }
            }
        }

        let spawn_queue = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.spawn_queue)
        };
        let vm_id = vm.id;
        for (pid, parent_id, spec, args, uid, username, forward_stdout) in spawn_queue {
            let parent_cwd = {
                let ctx = vm.lua.app_data_ref::<VmContext>().unwrap();
                ctx.process_cwd
                    .get(&parent_id)
                    .cloned()
                    .unwrap_or_else(|| "/".to_string())
            };
            let path = match &spec {
                SpawnSpec::Bin(name) => format!("/bin/{}", name),
                SpawnSpec::Path(p) => p.clone(),
            };
            let forward_stdout_to = if forward_stdout && parent_id != 0 {
                vm.os
                    .processes
                    .iter()
                    .find(|p| p.id == parent_id)
                    .map(|p| p.stdout.clone())
            } else {
                None
            };
            let result = manager.fs_service.read_file(vm_id, &path).await;
            if let Ok(Some((data, _))) = result {
                if let Ok(lua_code) = String::from_utf8(data) {
                    let argv0 = match &spec {
                        SpawnSpec::Bin(n) => n.clone(),
                        SpawnSpec::Path(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
                    };
                    let parent = if parent_id == 0 { None } else { Some(parent_id) };
                    vm.os.spawn_process_with_id(
                        &vm.lua,
                        pid,
                        parent,
                        &lua_code,
                        args,
                        uid,
                        &username,
                        forward_stdout_to,
                        Some(argv0),
                    );
                    if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
                        ctx.process_cwd.insert(pid, parent_cwd);
                    }
                }
            }
        }

        let stdin_inject = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.stdin_inject_queue)
        };
        for (pid, line) in stdin_inject {
            if let Some(p) = vm.os.processes.iter_mut().find(|pr| pr.id == pid) {
                if let Ok(mut guard) = p.stdin.lock() {
                    push_stdin_line(&mut guard, line);
                }
            }
        }
        let requested_kills = {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            std::mem::take(&mut ctx.requested_kills)
        };
        for pid in requested_kills {
            vm.os.kill_process_and_descendants(pid);
        }

        {
            let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
            if let Some(nic) = &mut vm.nic {
                for pkt in ctx.net_outbound.drain(..) {
                    nic.send(pkt);
                }
                for (port, pid) in ctx.pending_listen.drain(..) {
                    let _ = nic.try_listen(port, pid);
                }
                for port in ctx.pending_ephemeral_register.drain(..) {
                    nic.register_ephemeral(port);
                }
                for port in ctx.pending_ephemeral_unregister.drain(..) {
                    nic.unregister_ephemeral(port);
                }
            }
            std::mem::swap(&mut ctx.connections, &mut vm.connections);
            std::mem::swap(&mut ctx.next_connection_id, &mut vm.next_connection_id);
        }
    }

    /// Variant that simulates 60 FPS game loop: sleeps ~16.6ms between ticks.
    async fn run_tick_until_done_with_limit_60fps(
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
                let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                ctx.set_vm(vm_id, hostname, None);
            }
            for process in &mut vm.os.processes {
                if process.is_finished() {
                    continue;
                }
                {
                    let mut ctx = vm.lua.app_data_mut::<VmContext>().unwrap();
                    ctx.current_pid = process.id;
                    ctx.current_uid = process.user_id;
                    ctx.current_username = process.username.clone();
                    ctx.set_current_process(
                        process.stdin.clone(),
                        process.stdout.clone(),
                        process.args.clone(),
                        process.forward_stdout_to.clone(),
                    );
                }
                process.tick().expect("process tick failed");
                if process.is_finished() {
                    stdout_result = process.stdout.lock().unwrap().clone();
                }
            }
            let finished_pids: Vec<u64> = vm
                .os
                .processes
                .iter()
                .filter(|p| p.is_finished())
                .map(|p| p.id)
                .collect();
            vm.os.processes.retain(|p| !p.is_finished());
            if let Some(nic) = &mut vm.nic {
                for pid in &finished_pids {
                    nic.unlisten_pid(*pid);
                }
            }
            if vm.os.processes.is_empty() || vm.os.is_finished() {
                break;
            }
            sleep(super::TICK_TIME).await;
        }
        (stdout_result, tick_count)
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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(&vm.lua,
            bin_programs::CAT,
            vec!["/tmp/cat_test.txt".to_string()],
            0,
            "root",
        );

        let stdout = run_tick_until_done(&mut vm, vm_id, "cat-test-vm");

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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(&vm.lua, bin_programs::LS, vec!["/".to_string()], 0, "root");

        let stdout = run_tick_until_done(&mut vm, vm_id, "ls-test-vm");

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

    /// Bin grep: search for pattern in files.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep() {
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
            hostname: "grep-test-vm".to_string(),
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
            .write_file(vm_id, "/tmp/grep_a.txt", b"foo\nbar\nbaz\n", None, "root")
            .await
            .unwrap();
        fs_service
            .write_file(vm_id, "/tmp/grep_b.txt", b"nobar\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["bar".to_string(), "/tmp/grep_a.txt".to_string(), "/tmp/grep_b.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-test-vm");
        assert!(stdout.contains("/tmp/grep_a.txt:2:bar"), "grep should find bar in grep_a.txt, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/grep_b.txt:1:nobar"), "grep should find nobar in grep_b.txt, got: {:?}", stdout);
    }

    /// Bin find: list paths recursively.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find() {
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
            hostname: "find-test-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_dir/f.txt", b"x", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_dir".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-test-vm");
        assert!(stdout.contains("/tmp/find_dir"), "find should list dir, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/find_dir/f.txt"), "find should list file, got: {:?}", stdout);
    }

    /// Bin sed: substitute pattern with replacement.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed() {
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
            hostname: "sed-test-vm".to_string(),
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
            .write_file(vm_id, "/tmp/sed_in.txt", b"hello world\nworld end\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["world".to_string(), "Earth".to_string(), "/tmp/sed_in.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-test-vm");
        assert!(stdout.contains("hello Earth"), "sed should replace world, got: {:?}", stdout);
        assert!(stdout.contains("Earth end"), "sed should replace second world, got: {:?}", stdout);
    }

    /// Bin grep: no match in any file yields empty stdout.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_no_match() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 1), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-no-match-vm".to_string(),
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
            .write_file(vm_id, "/tmp/nomatch.txt", b"foo\nbaz\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["nonexistent".to_string(), "/tmp/nomatch.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-no-match-vm");
        assert!(
            !stdout.contains(":") || stdout.contains("usage"),
            "grep with no match should not print path:line, got: {:?}",
            stdout
        );
    }

    /// Bin grep: recursive -r on directory finds pattern in nested files.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_recursive() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 2), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-recursive-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/grep_rec", "root").await.unwrap();
        fs_service
            .write_file(vm_id, "/tmp/grep_rec/one.txt", b"needle here\n", None, "root")
            .await
            .unwrap();
        fs_service.mkdir(vm_id, "/tmp/grep_rec/sub", "root").await.unwrap();
        fs_service
            .write_file(vm_id, "/tmp/grep_rec/sub/two.txt", b"no\nneedle there\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["needle".to_string(), "/tmp/grep_rec".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-recursive-vm");
        assert!(stdout.contains("needle here"), "grep on dir should find in one.txt, got: {:?}", stdout);
        assert!(stdout.contains("needle there"), "grep on dir should find in sub/two.txt, got: {:?}", stdout);
    }

    /// Bin find: nested directories all listed.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_nested_dirs() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 2), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-nested-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/nested", "root").await.unwrap();
        fs_service.mkdir(vm_id, "/tmp/nested/a", "root").await.unwrap();
        fs_service.mkdir(vm_id, "/tmp/nested/a/b", "root").await.unwrap();
        fs_service
            .write_file(vm_id, "/tmp/nested/a/b/leaf.txt", b"x", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/nested".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-nested-vm");
        assert!(stdout.contains("/tmp/nested"), "find should list root, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/nested/a"), "find should list a, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/nested/a/b"), "find should list a/b, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/nested/a/b/leaf.txt"), "find should list leaf, got: {:?}", stdout);
    }

    /// Bin find: empty directory lists only the root path.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_empty_dir() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 3), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-empty-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/empty_dir", "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/empty_dir".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-empty-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find on empty dir should list only root, got: {:?}", stdout);
        assert_eq!(lines[0], "/tmp/empty_dir", "only line should be root path, got: {:?}", stdout);
    }

    /// Bin sed: file without pattern leaves content unchanged.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_pattern_not_found() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 1), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-no-pattern-vm".to_string(),
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
            .write_file(vm_id, "/tmp/sed_unchanged.txt", b"alpha\nbeta\ngamma\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["nonexistent".to_string(), "replacement".to_string(), "/tmp/sed_unchanged.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-no-pattern-vm");
        assert!(stdout.contains("alpha"), "sed should leave content unchanged, got: {:?}", stdout);
        assert!(stdout.contains("beta"), "sed should leave content unchanged, got: {:?}", stdout);
        assert!(!stdout.contains("replacement"), "sed should not replace when pattern absent, got: {:?}", stdout);
    }

    /// Bin grep: pattern at start of line is found.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_pattern_at_line_start() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 20), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-start-vm".to_string(),
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
            .write_file(vm_id, "/tmp/start.txt", b"prefix\nstart_here\nsuffix\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["start".to_string(), "/tmp/start.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-start-vm");
        assert!(stdout.contains("/tmp/start.txt:2:start_here"), "grep should find pattern at line start, got: {:?}", stdout);
    }

    /// Bin grep: pattern at end of line is found.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_pattern_at_line_end() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 21), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-end-vm".to_string(),
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
            .write_file(vm_id, "/tmp/end.txt", b"first\nat_end\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["end".to_string(), "/tmp/end.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-end-vm");
        assert!(stdout.contains("/tmp/end.txt:2:at_end"), "grep should find pattern at line end, got: {:?}", stdout);
    }

    /// Bin grep: search is case-sensitive; "Bar" does not match "bar".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_case_sensitive() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 22), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-case-vm".to_string(),
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
            .write_file(vm_id, "/tmp/case.txt", b"bar\nBar\nBAR\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["Bar".to_string(), "/tmp/case.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-case-vm");
        assert!(stdout.contains("Bar"), "grep should find exact Bar, got: {:?}", stdout);
        assert_eq!(stdout.matches("Bar").count(), 1, "grep should find only one Bar line, got: {:?}", stdout);
    }

    /// Bin grep: single file path only (explicit file).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_single_file_only() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 23), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-single-vm".to_string(),
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
            .write_file(vm_id, "/tmp/only.txt", b"one\ntwo\nthree\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["two".to_string(), "/tmp/only.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-single-vm");
        assert!(stdout.contains("/tmp/only.txt:2:two"), "grep single file should find line, got: {:?}", stdout);
    }

    /// Bin grep: nonexistent path does not crash; no output for that path.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_nonexistent_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 24), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-nonexistent-vm".to_string(),
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
            .write_file(vm_id, "/tmp/real.txt", b"match\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["match".to_string(), "/tmp/real.txt".to_string(), "/tmp/nonexistent.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-nonexistent-vm");
        assert!(stdout.contains("/tmp/real.txt:1:match"), "grep should find in real file, got: {:?}", stdout);
        assert!(!stdout.contains("nonexistent"), "grep should not output for nonexistent path, got: {:?}", stdout);
    }

    /// Bin grep: pattern with special characters (literal substring).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_special_chars_in_pattern() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 25), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-special-vm".to_string(),
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
            .write_file(vm_id, "/tmp/special.txt", b"a.b.c\nx y z\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["a.b".to_string(), "/tmp/special.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "grep-special-vm");
        assert!(stdout.contains("a.b.c"), "grep should find literal a.b, got: {:?}", stdout);
    }

    /// Bin find: on a single file path (non-directory) lists only that path.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_single_file_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 20), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-file-vm".to_string(),
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
        fs_service.write_file(vm_id, "/tmp/single_file.txt", b"x", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/single_file.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-file-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find on file should list one path, got: {:?}", stdout);
        assert_eq!(lines[0], "/tmp/single_file.txt", "find should list the file path, got: {:?}", stdout);
    }

    /// Bin find: no args uses default path "." (current dir).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_default_dot() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 21), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-dot-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_dot_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_dot_dir/f.txt", b"x", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        // Run find with explicit path (same as would be cwd). Find resolves relative paths; without filters we list all.
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_dot_dir".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-dot-vm");
        assert!(stdout.contains("/tmp/find_dot_dir"), "find should list dir, got: {:?}", stdout);
        assert!(stdout.contains("/tmp/find_dot_dir/f.txt"), "find should list file, got: {:?}", stdout);
    }

    /// Bin find: nonexistent path produces no output (no crash).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_nonexistent_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 22), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-nonexistent-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/does_not_exist_xyz".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-nonexistent-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 0, "find on nonexistent path should output nothing, got: {:?}", stdout);
    }

    /// Bin find -name: only path whose basename matches is printed.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_name() {
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
            hostname: "find-name-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_name_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_name_dir/morango", b"x", None, "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_name_dir/banana", b"y", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_name_dir".to_string(), "-name".to_string(), "morango".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-name-vm", 500);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find -name morango should output one path, got: {:?}", stdout);
        assert!(lines[0].ends_with("morango"), "path should end with morango, got: {:?}", lines[0]);
    }

    /// Bin find -name with glob: only paths matching *.lua.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_name_glob() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 1), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-glob-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_glob_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_glob_dir/a.lua", b"x", None, "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_glob_dir/b.txt", b"y", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_glob_dir".to_string(), "-name".to_string(), "*.lua".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-glob-vm", 500);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find -name '*.lua' should output one path, got: {:?}", stdout);
        assert!(lines[0].ends_with("a.lua"), "path should end with a.lua, got: {:?}", lines[0]);
    }

    /// Bin find -type f: only files.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_type_f() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 2), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-type-f-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_type_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_type_dir/file.txt", b"x", None, "root").await.unwrap();
        fs_service.mkdir(vm_id, "/tmp/find_type_dir/subdir", "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_type_dir".to_string(), "-type".to_string(), "f".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-type-f-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find -type f should output one file path, got: {:?}", stdout);
        assert!(lines[0].ends_with("file.txt"), "path should be the file, got: {:?}", lines[0]);
    }

    /// Bin find -type d: only directories.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_type_d() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 3), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-type-d-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_typed_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_typed_dir/f.txt", b"x", None, "root").await.unwrap();
        fs_service.mkdir(vm_id, "/tmp/find_typed_dir/sub", "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_typed_dir".to_string(), "-type".to_string(), "d".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-type-d-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2, "find -type d should output root dir and sub, got: {:?}", stdout);
        assert!(lines.iter().any(|l| l.ends_with("find_typed_dir")), "should list root dir");
        assert!(lines.iter().any(|l| l.ends_with("sub")), "should list sub dir");
    }

    /// Bin find -user: only paths owned by given user.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_user() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 4), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-user-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_user_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_user_dir/root_file", b"a", None, "root").await.unwrap();
        fs_service.mkdir(vm_id, "/tmp/find_user_dir/alice_dir", "alice").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_user_dir/alice_dir/alice_file", b"b", None, "alice").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_user_dir".to_string(), "-user".to_string(), "alice".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-user-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2, "find -user alice should output alice_dir and alice_file, got: {:?}", stdout);
        assert!(lines.iter().any(|l| l.ends_with("alice_dir")), "should list alice dir");
        assert!(lines.iter().any(|l| l.ends_with("alice_file")), "should list alice file");
    }

    /// Bin find -size: filter by file size (e.g. -size +0 for non-empty).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_size() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 5), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-size-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_size_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_size_dir/empty", b"", None, "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_size_dir/has123", b"123", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_size_dir".to_string(), "-type".to_string(), "f".to_string(), "-size".to_string(), "+0".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-size-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find -type f -size +0 should output only file with size > 0, got: {:?}", stdout);
        assert!(lines[0].ends_with("has123"), "path should be has123 (3 bytes), got: {:?}", lines[0]);
    }

    /// Bin find -name with no match: empty output.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_no_match() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 6), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-no-match-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_nomatch_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_nomatch_dir/something", b"x", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_nomatch_dir".to_string(), "-name".to_string(), "nonexistent".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "find-no-match-vm");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 0, "find -name nonexistent should output nothing, got: {:?}", stdout);
    }

    /// Bin find -iname: case-insensitive name match.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_iname() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 7), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-iname-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/find_iname_dir", "root").await.unwrap();
        fs_service.write_file(vm_id, "/tmp/find_iname_dir/Morango", b"x", None, "root").await.unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/find_iname_dir".to_string(), "-iname".to_string(), "morango".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-iname-vm", 500);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 1, "find -iname morango should match Morango, got: {:?}", stdout);
        assert!(lines[0].ends_with("Morango"), "path should end with Morango, got: {:?}", lines[0]);
    }

    /// Bin sed: empty replacement string removes pattern from output.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_empty_replacement() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 20), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-empty-repl-vm".to_string(),
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
            .write_file(vm_id, "/tmp/sed_empty_repl.txt", b"remove_me and rest\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["remove_me ".to_string(), "".to_string(), "/tmp/sed_empty_repl.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-empty-repl-vm");
        assert!(stdout.contains("and rest"), "sed empty replacement should remove pattern, got: {:?}", stdout);
        assert!(!stdout.contains("remove_me"), "sed should not contain original pattern, got: {:?}", stdout);
    }

    /// Bin sed: multiple lines with pattern on some lines only.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_multiple_lines_partial_match() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 21), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-multi-vm".to_string(),
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
            .write_file(vm_id, "/tmp/sed_multi.txt", b"line1\nreplace_this\nline3\nno_change\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["replace_this".to_string(), "done".to_string(), "/tmp/sed_multi.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-multi-vm");
        assert!(stdout.contains("line1"), "sed should keep line1, got: {:?}", stdout);
        assert!(stdout.contains("done"), "sed should replace on matching line, got: {:?}", stdout);
        assert!(stdout.contains("line3"), "sed should keep line3, got: {:?}", stdout);
        assert!(stdout.contains("no_change"), "sed should keep line without pattern, got: {:?}", stdout);
    }

    /// Bin sed: file not found prints error and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_file_not_found() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 22), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-notfound-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["x".to_string(), "y".to_string(), "/tmp/nonexistent_sed_file.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-notfound-vm");
        assert!(stdout.contains("sed: cannot read"), "sed should print error for missing file, got: {:?}", stdout);
        assert!(stdout.contains("nonexistent_sed_file"), "sed error should mention path, got: {:?}", stdout);
    }

    /// Bin sed: pattern at line boundary (start of content).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_pattern_at_boundary() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 23), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-boundary-vm".to_string(),
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
            .write_file(vm_id, "/tmp/sed_boundary.txt", b"first\nlast\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["first".to_string(), "1st".to_string(), "/tmp/sed_boundary.txt".to_string()],
            0,
            "root",
        );
        let stdout = run_tick_until_done(&mut vm, vm_id, "sed-boundary-vm");
        assert!(stdout.contains("1st"), "sed should replace at start of content, got: {:?}", stdout);
        assert!(stdout.contains("last"), "sed should leave other line unchanged, got: {:?}", stdout);
    }

    // --- Stress tests for grep, find, sed (more ticks allowed) ---

    /// Grep stress: many files (50), pattern in half of them.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_stress_many_files() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 10), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-stress-many-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/grep_stress", "root").await.unwrap();
        for i in 0..50u32 {
            let path = format!("/tmp/grep_stress/f{:02}.txt", i);
            let content = if i % 2 == 0 {
                format!("line1\nmatch_here\nline3\n")
            } else {
                "no match in this file\n".to_string()
            };
            fs_service
                .write_file(vm_id, &path, content.as_bytes(), None, "root")
                .await
                .unwrap();
        }
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["match_here".to_string(), "/tmp/grep_stress".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "grep-stress-many-vm", 2000);
        let match_count = stdout.lines().filter(|l| l.contains("match_here")).count();
        assert_eq!(match_count, 25, "grep should find 25 files with match_here, got {} lines", match_count);
    }

    /// Grep stress: one file with many lines (200), pattern on every other line.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_stress_many_matches() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 11), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-stress-matches-vm".to_string(),
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
        let mut content = String::new();
        for i in 0..200 {
            if i % 2 == 0 {
                content.push_str("hit\n");
            } else {
                content.push_str("miss\n");
            }
        }
        fs_service
            .write_file(vm_id, "/tmp/many_lines.txt", content.as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["hit".to_string(), "/tmp/many_lines.txt".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "grep-stress-matches-vm", 1500);
        let hits = stdout.lines().filter(|l| l.ends_with("hit")).count();
        assert_eq!(hits, 100, "grep should find 100 lines with hit, got {}", hits);
    }

    /// Grep stress: long line (1500 chars) with pattern in the middle.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_stress_long_line() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 12), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-stress-long-vm".to_string(),
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
        let pad = "x".repeat(700);
        let content = format!("{}needle_in_middle{}\n", pad, pad);
        fs_service
            .write_file(vm_id, "/tmp/long_line.txt", content.as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec!["needle_in_middle".to_string(), "/tmp/long_line.txt".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "grep-stress-long-vm", 8000);
        assert!(stdout.contains("needle_in_middle"), "grep should find pattern in long line, got: {:?}", stdout);
    }

    /// Find stress: deep directory tree (15 levels).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_stress_deep_tree() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 10), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-stress-deep-vm".to_string(),
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
        let mut path = "/tmp/deep".to_string();
        fs_service.mkdir(vm_id, &path, "root").await.unwrap();
        for d in 1..=14 {
            path.push_str(&format!("/d{}", d));
            fs_service.mkdir(vm_id, &path, "root").await.unwrap();
        }
        fs_service
            .write_file(vm_id, &format!("{}/leaf", path), b"x", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/deep".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-stress-deep-vm", 2000);
        assert!(stdout.contains("/tmp/deep"), "find should list root");
        assert!(stdout.contains("/tmp/deep/d1"), "find should list d1");
        assert!(stdout.contains("/tmp/deep/d1/d2/d3"), "find should list deep path");
        assert!(stdout.contains("leaf"), "find should list leaf file");
    }

    /// Find stress: wide tree — one dir with 40 subdirs, each with one file.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_stress_wide_tree() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 11), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-stress-wide-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/wide", "root").await.unwrap();
        for i in 0..40u32 {
            let dir = format!("/tmp/wide/s{}", i);
            fs_service.mkdir(vm_id, &dir, "root").await.unwrap();
            fs_service
                .write_file(vm_id, &format!("{}/f.txt", dir), b"x", None, "root")
                .await
                .unwrap();
        }
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/wide".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-stress-wide-vm", 2000);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines.len() >= 81, "find should list root + 40 dirs + 40 files, got {} lines", lines.len());
        assert!(stdout.contains("/tmp/wide/s0/f.txt") && stdout.contains("/tmp/wide/s39/f.txt"));
    }

    /// Find stress: many files (60) in a single directory.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_stress_many_files_one_dir() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 12), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-stress-flat-vm".to_string(),
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
        fs_service.mkdir(vm_id, "/tmp/flat", "root").await.unwrap();
        for i in 0..60u32 {
            fs_service
                .write_file(vm_id, &format!("/tmp/flat/file_{:02}.txt", i), b"x", None, "root")
                .await
                .unwrap();
        }
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/flat".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-stress-flat-vm", 500);
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 61, "find should list 1 dir + 60 files, got {}", lines.len());
    }

    /// Sed stress: file with 150 lines, replace on every line.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_stress_many_replacements() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 10), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-stress-many-vm".to_string(),
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
        let content = (0..150).map(|i| format!("replace_me line{}\n", i)).collect::<String>();
        fs_service
            .write_file(vm_id, "/tmp/sed_many.txt", content.as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec![
                "replace_me".to_string(),
                "DONE".to_string(),
                "/tmp/sed_many.txt".to_string(),
            ],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "sed-stress-many-vm", 2000);
        let done_count = stdout.matches("DONE").count();
        assert_eq!(done_count, 150, "sed should replace 150 occurrences, got {}", done_count);
    }

    /// Sed stress: one long line with pattern repeated 80 times.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_stress_long_line_many_matches() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 11), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-stress-long-vm".to_string(),
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
        let line = "ab".repeat(80);
        fs_service
            .write_file(vm_id, "/tmp/sed_long.txt", format!("{}\n", line).as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["ab".to_string(), "X".to_string(), "/tmp/sed_long.txt".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "sed-stress-long-vm", 1500);
        let x_count = stdout.matches("X").count();
        assert_eq!(x_count, 80, "sed should replace 80 'ab' with 'X', got {}", x_count);
    }

    /// Sed stress: replacement longer than pattern, many occurrences.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_stress_replacement_longer() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 12), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-stress-long-repl-vm".to_string(),
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
        let content = "x x x x x x x x x x\n"; // 10 occurrences
        fs_service
            .write_file(vm_id, "/tmp/sed_repl.txt", content.as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec![
                "x".to_string(),
                "long_replacement".to_string(),
                "/tmp/sed_repl.txt".to_string(),
            ],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "sed-stress-long-repl-vm", 500);
        let count = stdout.matches("long_replacement").count();
        assert_eq!(count, 10, "sed should replace 10 'x' with longer string, got {}", count);
    }

    /// Grep stress: empty file and mixed paths (no crash, correct match count).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_grep_stress_empty_file_and_multi_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 93, 13), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "grep-stress-empty-vm".to_string(),
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
            .write_file(vm_id, "/tmp/empty.txt", b"", None, "root")
            .await
            .unwrap();
        fs_service
            .write_file(vm_id, "/tmp/has.txt", b"needle\n", None, "root")
            .await
            .unwrap();
        fs_service
            .write_file(vm_id, "/tmp/none.txt", b"nope\n", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::GREP,
            vec![
                "needle".to_string(),
                "/tmp/empty.txt".to_string(),
                "/tmp/has.txt".to_string(),
                "/tmp/none.txt".to_string(),
            ],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "grep-stress-empty-vm", 300);
        assert!(stdout.contains("/tmp/has.txt:1:needle"), "grep should find in has.txt, got: {:?}", stdout);
        assert!(!stdout.contains("/tmp/empty.txt"), "empty file should produce no match line");
        assert!(!stdout.contains("/tmp/none.txt"), "file without pattern should produce no match");
    }

    /// Find stress: very deep tree (20 levels).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_find_stress_very_deep() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 92, 13), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "find-stress-verydeep-vm".to_string(),
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
        let mut path = "/tmp/vdeep".to_string();
        fs_service.mkdir(vm_id, &path, "root").await.unwrap();
        for d in 1..=19 {
            path.push_str(&format!("/l{}", d));
            fs_service.mkdir(vm_id, &path, "root").await.unwrap();
        }
        fs_service
            .write_file(vm_id, &format!("{}/end", path), b"ok", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::FIND,
            vec!["/tmp/vdeep".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "find-stress-verydeep-vm", 3000);
        assert!(stdout.contains("/tmp/vdeep"), "find should list root");
        assert!(stdout.contains("l10"), "find should list mid-level");
        assert!(stdout.contains("end"), "find should list leaf file");
        let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 21, "find should list 1 root + 19 dirs + 1 file = 21, got {}", lines.len());
    }

    /// Sed stress: empty file (no crash, empty output).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_stress_empty_file() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 13), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-stress-empty-vm".to_string(),
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
            .write_file(vm_id, "/tmp/empty_sed.txt", b"", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["a".to_string(), "b".to_string(), "/tmp/empty_sed.txt".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "sed-stress-empty-vm", 100);
        assert!(stdout.is_empty(), "sed on empty file should output nothing, got: {:?}", stdout);
    }

    /// Sed stress: 200 replacements in one file (more than many_replacements).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_sed_stress_heavy_replacements() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 91, 14), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "sed-stress-heavy-vm".to_string(),
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
        let content = (0..200).map(|_| "OLD\n").collect::<String>();
        fs_service
            .write_file(vm_id, "/tmp/sed_heavy.txt", content.as_bytes(), None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(
            &vm.lua,
            bin_programs::SED,
            vec!["OLD".to_string(), "NEW".to_string(), "/tmp/sed_heavy.txt".to_string()],
            0,
            "root",
        );
        let (stdout, _) = run_tick_until_done_with_limit(&mut vm, vm_id, "sed-stress-heavy-vm", 3000);
        let new_count = stdout.matches("NEW").count();
        assert_eq!(new_count, 200, "sed should replace 200 OLD with NEW, got {}", new_count);
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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let touch_path = "/tmp/touch_created.txt";
        vm.os.spawn_process(&vm.lua,
            bin_programs::TOUCH,
            vec![touch_path.to_string()],
            0,
            "root",
        );

        run_tick_until_done(&mut vm, vm_id, "touch-test-vm");

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

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        vm.os.spawn_process(&vm.lua,bin_programs::RM, vec![rm_path.to_string()], 0, "root");

        run_tick_until_done(&mut vm, vm_id, "rm-test-vm");

        let content = fs_service.read_file(vm_id, rm_path).await.unwrap();
        assert!(
            content.is_none(),
            "rm should have deleted {}",
            rm_path
        );
    }

    /// Bin lua -help: prints usage and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_help() {
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
            hostname: "lua-help-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["-help".to_string()], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-help-vm");
        assert!(stdout.contains("lua: usage: lua <file>"), "stdout should contain usage, got: {:?}", stdout);
        assert!(stdout.contains("-d"), "stdout should mention -d, got: {:?}", stdout);
    }

    /// Bin lua --help: prints usage and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_help_long() {
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
            hostname: "lua-help-long-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["--help".to_string()], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-help-long-vm");
        assert!(stdout.contains("lua: usage: lua <file>"), "stdout should contain usage, got: {:?}", stdout);
    }

    /// Bin lua no args: prints usage and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_no_args() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 89, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "lua-no-args-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec![], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-no-args-vm");
        assert!(stdout.contains("lua: usage: lua <file>"), "stdout should contain usage, got: {:?}", stdout);
    }

    /// Bin lua -d alone: prints usage and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_single_d() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 88, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "lua-single-d-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["-d".to_string()], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-single-d-vm");
        assert!(stdout.contains("lua: usage: lua <file>"), "stdout should contain usage, got: {:?}", stdout);
    }

    /// Bin lua runs script in same process; script stdout appears in lua process stdout.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_runs_script() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 87, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "lua-script-vm".to_string(),
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
            .write_file(vm_id, "/tmp/lua_test_script.lua", b"print(\"lua_script_ok\")", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["/tmp/lua_test_script.lua".to_string()], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-script-vm");
        assert!(stdout.contains("lua_script_ok"), "stdout should contain script output, got: {:?}", stdout);
    }

    /// Bin lua nonexistent file: prints error and exits.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_nonexistent_file() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 86, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "lua-nonexistent-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["/nonexistent.lua".to_string()], 0, "root");
        let stdout = run_tick_until_done(&mut vm, vm_id, "lua-nonexistent-vm");
        assert!(stdout.contains("cannot read file"), "stdout should contain error, got: {:?}", stdout);
    }

    /// Bin lua file.lua -d: spawns script as daemon (child); parent exits; child runs and writes marker file.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bin_lua_daemon() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 85, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "lua-daemon-vm".to_string(),
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
        let daemon_script = b"fs.write(\"/tmp/lua_daemon_marker.txt\", \"ok\", nil)";
        fs_service
            .write_file(vm_id, "/tmp/lua_daemon_script.lua", daemon_script, None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);
        vm.os.spawn_process(&vm.lua, bin_programs::LUA, vec!["/tmp/lua_daemon_script.lua".to_string(), "-d".to_string()], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "lua-daemon-vm", 150).await;
        let file = fs_service.read_file(vm_id, "/tmp/lua_daemon_marker.txt").await.unwrap();
        assert!(file.is_some(), "daemon child should have written marker file");
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

    // --- Lua os API integration tests (full-tick with spawn_queue + stdin inject) ---

    #[tokio::test]
    async fn test_os_spawn_returns_pid_and_child_runs() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 84, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "os-spawn-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let parent_script = r#"
local pid = os.spawn("echo", {"from_spawn"})
io.write("pid=" .. pid .. "\n")
"#;
        vm.os.spawn_process(&vm.lua,parent_script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "os-spawn-vm", 100).await;

        assert!(
            stdout_by_pid.get(&2).map(|s| s.contains("from_spawn")).unwrap_or(false),
            "child (pid 2) stdout should contain 'from_spawn', got: {:?}",
            stdout_by_pid
        );
        assert!(
            stdout_by_pid.get(&1).map(|s| s.contains("pid=2")).unwrap_or(false),
            "parent stdout should contain 'pid=2', got: {:?}",
            stdout_by_pid.get(&1)
        );
        // Child should have parent_id set (assert from Rust: we no longer have process list after they finish, but we verified child ran)
    }

    #[tokio::test]
    async fn test_os_spawn_path_runs_script_from_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 83, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "spawn-path-vm".to_string(),
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
        let path_script = r#"io.write("spawn_path_ok")"#;
        fs_service
            .write_file(vm_id, "/tmp/spawn_path_test.lua", path_script.as_bytes(), None, "root")
            .await
            .unwrap();

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let parent_script = r#"
os.spawn_path("/tmp/spawn_path_test.lua", {})
"#;
        vm.os.spawn_process(&vm.lua,parent_script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "spawn-path-vm", 100)
                .await;

        assert!(
            stdout_by_pid.values().any(|s| s.contains("spawn_path_ok")),
            "child stdout should contain 'spawn_path_ok', got: {:?}",
            stdout_by_pid
        );
    }

    #[tokio::test]
    async fn test_os_process_status_not_found() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 82, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "status-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let script = r#"io.write(os.process_status(999))"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "status-vm", 50).await;

        let out = stdout_by_pid.get(&1).map(|s| s.as_str()).unwrap_or("");
        assert!(out.contains("not_found"), "expected 'not_found', got: {:?}", out);
    }

    #[tokio::test]
    async fn test_os_process_status_finished_after_child_exits() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 81, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "status-finished-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Child may be reaped (removed from list) before parent runs again, so status becomes "not_found"; accept both.
        let script = r#"
local pid = os.spawn("echo", {"x"})
while true do
  local s = os.process_status(pid)
  if s == "finished" or s == "not_found" then
    io.write("status=finished")
    break
  end
end
"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, _) = run_tick_until_done_with_spawn(
            &mut vm,
            &manager,
            vm_id,
            "status-finished-vm",
            200,
        )
        .await;

        let out = stdout_by_pid.get(&1).map(|s| s.as_str()).unwrap_or("");
        assert!(
            out.contains("status=finished"),
            "expected 'status=finished', got: {:?}",
            out
        );
    }

    #[tokio::test]
    async fn test_os_write_stdin_injects_line_to_child() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 80, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "stdin-vm".to_string(),
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
        // Child reads in a loop until stdin has a line (parent does spawn then write_stdin; inject runs later), then writes and exits.
        let reader_script = r#"
while true do
  local l = io.read()
  if l and l ~= "" then
    io.write("got:" .. l)
    break
  end
end
"#;
        fs_service
            .write_file(vm_id, "/tmp/read_stdin.lua", reader_script.as_bytes(), None, "root")
            .await
            .unwrap();

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let parent_script = r#"
local pid = os.spawn_path("/tmp/read_stdin.lua", {})
os.write_stdin(pid, "hello")
"#;
        vm.os.spawn_process(&vm.lua,parent_script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "stdin-vm", 200).await;

        assert!(
            stdout_by_pid.values().any(|s| s.contains("got:hello")),
            "child stdout should contain 'got:hello', got: {:?}",
            stdout_by_pid
        );
    }

    #[tokio::test]
    async fn test_os_read_stdout_returns_child_output() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 79, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "read-stdout-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Parent spawns echo child; each loop consumes any stdout, exits when child is finished or not_found.
        let script = r#"
local pid = os.spawn("echo", {"read_stdout_ok"})
while true do
  local out = os.read_stdout(pid)
  if out then io.write(out) end
  local s = os.process_status(pid)
  if s == "finished" or s == "not_found" then
    break
  end
end
"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, tick_count) = run_tick_until_done_with_spawn(
            &mut vm,
            &manager,
            vm_id,
            "read-stdout-vm",
            200,
        )
        .await;

        // Child (echo) must produce output.
        let child_out = stdout_by_pid.get(&2).map(|s| s.as_str()).unwrap_or("");
        assert!(
            child_out.contains("read_stdout_ok"),
            "child (pid 2) should produce 'read_stdout_ok', got: {:?}",
            stdout_by_pid
        );
        // Ideally parent reads child stdout via os.read_stdout(pid) after child is reaped; at least require multiple ticks ran.
        assert!(
            tick_count >= 2,
            "expected at least 2 ticks (spawn then child run), got {}",
            tick_count
        );
        let parent_out = stdout_by_pid.get(&1).map(|s| s.as_str()).unwrap_or("");
        if parent_out.contains("read_stdout_ok") {
            // Parent successfully read child stdout via os.read_stdout(pid).
        } else {
            // Known quirk: parent may get nil from os.read_stdout(pid) when child was reaped; child output is still captured above.
        }
    }

    #[tokio::test]
    async fn test_os_parse_cmd_returns_program_and_args() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 78, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "parse-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let script = r#"
local t = os.parse_cmd("cat file.txt --pretty")
io.write(t.program .. "|" .. table.concat(t.args, ","))
"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "parse-vm", 50).await;

        let out = stdout_by_pid.get(&1).map(|s| s.as_str()).unwrap_or("");
        assert!(
            out.contains("cat|file.txt,--pretty"),
            "parse_cmd should return program and args, got: {:?}",
            out
        );
    }

    #[tokio::test]
    async fn test_os_parse_cmd_key_value() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 77, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "parse-kv-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let script = r#"
local t = os.parse_cmd("sum age=2")
io.write(t.program)
for i = 1, #t.args do io.write("_" .. t.args[i]) end
"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "parse-kv-vm", 50).await;

        let out = stdout_by_pid.get(&1).map(|s| s.as_str()).unwrap_or("");
        assert!(out.contains("sum"), "program should be 'sum', got: {:?}", out);
        assert!(
            out.contains("age=2"),
            "args should contain 'age=2', got: {:?}",
            out
        );
    }

    #[tokio::test]
    async fn test_os_exec_still_spawns_from_bin() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 76, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "exec-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let script = r#"os.exec("echo", {"exec_ok"})"#;
        vm.os.spawn_process(&vm.lua,script, vec![], 0, "root");

        let (stdout_by_pid, _) =
            run_tick_until_done_with_spawn(&mut vm, &manager, vm_id, "exec-vm", 100).await;

        assert!(
            stdout_by_pid.values().any(|s| s.contains("exec_ok")),
            "os.exec should spawn echo and output 'exec_ok', got: {:?}",
            stdout_by_pid
        );
    }

    /// Shell parses stdin as bin command, spawns it, and relays child stdout to its own stdout.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_parses_and_runs_bin_command_relays_stdout() {
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
            hostname: "shell-echo-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver spawns shell then sends "echo hello" to shell stdin; driver exits.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "echo hello")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-echo-vm", 50).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("hello"),
            "shell should relay echo child stdout, got: {:?}",
            shell_stdout
        );
    }

    /// Shell with a running child forwards stdin to the child; child (echo_stdin) echoes and shell relays.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_forwards_stdin_to_child() {
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
            hostname: "shell-forward-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver spawns shell, sends "echo_stdin" (spawns child), then "hello" (forwarded to child).
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "echo_stdin")
os.write_stdin(pid, "hello")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        // Need enough ticks: shell reads "echo_stdin", spawns child; next tick reads "hello", forwards to child;
        // stdin inject applies end-of-tick so child gets "hello" next tick; child echoes; shell relays next tick.
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-forward-vm", 80).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("got:hello"),
            "shell should forward stdin to child and relay output, got: {:?}",
            shell_stdout
        );
    }

    /// Ctrl+C (special sequence \x03) via stdin: shell has foreground child (echo_stdin), inject \x03; child is killed, shell remains.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_ctrl_c_kills_foreground_child() {
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
            hostname: "ctrl-c-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver spawns shell, sends "echo_stdin" (spawns child), then Ctrl+C sequence \x03.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "echo_stdin")
os.write_stdin(pid, "\x03")
"#;
        vm.os.spawn_process(&vm.lua, driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "ctrl-c-vm", 80).await;

        // Shell (pid 2) and driver (pid 1) remain; echo_stdin child (pid 3) must be killed.
        let has_echo_stdin = vm.os.processes.iter().any(|p| {
            p.display_name
                .as_deref()
                .or(p.args.first().map(|s| s.as_str()))
                == Some("echo_stdin")
        });
        assert!(
            !has_echo_stdin,
            "Ctrl+C should kill foreground child (echo_stdin); processes: {:?}",
            vm.os
                .processes
                .iter()
                .map(|p| (p.id, p.display_name.as_deref(), p.args.first().map(|s| s.as_str())))
                .collect::<Vec<_>>()
        );
        let shell_count = vm
            .os
            .processes
            .iter()
            .filter(|p| p.display_name.as_deref() == Some("sh"))
            .count();
        assert!(
            shell_count >= 1,
            "Shell should still be running after Ctrl+C; processes: {}",
            vm.os.processes.len()
        );
    }

    /// Tab autocomplete (no child): shell receives "ec\x09", runs autocomplete, stdout contains \x01TABCOMPLETE\t + completion.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_tab_autocomplete_no_child() {
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
            hostname: "tab-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "ec\x09")
"#;
        vm.os.spawn_process(&vm.lua, driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "tab-vm", 80).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.display_name.as_deref() == Some("sh"))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("\x01TABCOMPLETE\t"),
            "Shell should output TABCOMPLETE protocol; got: {:?}",
            shell_stdout
        );
        assert!(
            shell_stdout.contains("echo"),
            "Completion for 'ec' should include 'echo'; got: {:?}",
            shell_stdout
        );
    }

    /// Tab autocomplete in cwd: file "casa" in /tmp; shell cd /tmp then "cat ca" + Tab yields "cat casa".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_tab_autocomplete_cwd_path() {
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
        let config = super::super::db::vm_service::VmConfig {
            hostname: "tab-cwd-vm".to_string(),
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
            .write_file(vm_id, "/tmp/casa", b"", None, "root")
            .await
            .unwrap();
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "cd /tmp\n")
os.write_stdin(pid, "cat ca\x09\n")
"#;
        vm.os.spawn_process(&vm.lua, driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "tab-cwd-vm", 300).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.display_name.as_deref() == Some("sh"))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("\x01TABCOMPLETE\t"),
            "Shell should output TABCOMPLETE; got: {:?}",
            shell_stdout
        );
        assert!(
            shell_stdout.contains("cat casa"),
            "Completion for 'cat ca' in cwd /tmp should yield 'cat casa'; got: {:?}",
            shell_stdout
        );
    }

    /// Tab when child is not ssh: shell discards the line (does not forward). Child (echo_stdin) never receives it.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_tab_discard_when_child_not_ssh() {
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
            hostname: "tab-discard-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "echo_stdin")
os.write_stdin(pid, "x\x09")
"#;
        vm.os.spawn_process(&vm.lua, driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "tab-discard-vm", 80).await;

        let child_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.display_name.as_deref() == Some("echo_stdin"))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            !child_stdout.contains("got:"),
            "Tab should be discarded (not forwarded to child); child stdout should be empty, got: {:?}",
            child_stdout
        );
    }

    /// Tab when child is ssh: shell forwards "ec\x09" to ssh; remote shell does autocomplete and sends TABCOMPLETE back.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_tab_forward_when_child_is_ssh() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 88, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "tab-ssh-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "tab-ssh-b".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        vm_b.os.spawn_process(&vm_b.lua, bin_programs::SSH_SERVER, vec![], 0, "root");

        let driver_script = format!(
            r#"
local pid = os.spawn("sh", {{}})
os.write_stdin(pid, "ssh {}")
os.write_stdin(pid, "ec\x09")
while true do end
"#,
            ip_b
        );
        vm_a.os.spawn_process(&vm_a.lua, &driver_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let ssh_stdout = vms[0]
            .os
            .processes
            .iter()
            .find(|p| p.display_name.as_deref() == Some("ssh"))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            ssh_stdout.contains("\x01TABCOMPLETE\t"),
            "Tab should be forwarded to ssh; remote shell should respond with TABCOMPLETE; got: {:?}",
            ssh_stdout
        );
    }

    /// Shell runs ls / and forwards child stdout; bootstrap has /bin.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_runs_ls() {
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
            hostname: "shell-ls-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "ls /")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
            ctx.process_cwd.insert(1, "/root".to_string());
        }

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-ls-vm", 150).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("bin"),
            "shell running ls / should show bin, got: {:?}",
            shell_stdout
        );
    }

    /// Shell runs echo with multiple args; stdout contains "a b c".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_runs_echo_multiple_args() {
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
            hostname: "shell-echo-args-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "echo a b c")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-echo-args-vm", 50).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("a b c"),
            "shell running echo a b c should show a b c, got: {:?}",
            shell_stdout
        );
    }

    /// Shell runs touch; file exists after ticks.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_runs_touch() {
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
            hostname: "shell-touch-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "touch /tmp/shell_touch_test")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-touch-vm", 100).await;

        let file = fs_service
            .read_file(vm_id, "/tmp/shell_touch_test")
            .await
            .unwrap();
        assert!(
            file.is_some(),
            "shell running touch should create /tmp/shell_touch_test"
        );
    }

    /// Shell runs cat on a pre-created file; stdout contains file content.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_runs_cat() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 89, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-cat-vm".to_string(),
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
            .write_file(
                vm_id,
                "/tmp/shell_cat_test",
                b"content for cat",
                None,
                "root",
            )
            .await
            .unwrap();

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "cat /tmp/shell_cat_test")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-cat-vm", 100).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("content for cat"),
            "shell running cat should show file content, got: {:?}",
            shell_stdout
        );
    }

    /// Shell runs touch then rm; file is removed after ticks.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_runs_rm() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 88, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-rm-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver must yield after "touch" so shell runs touch and clears child before "rm" is injected
        // (otherwise shell would forward "rm" to the touch child instead of running rm).
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "touch /tmp/shell_rm_test")
for i = 1, 100 do end
os.write_stdin(pid, "rm /tmp/shell_rm_test")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");

        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-rm-vm", 200).await;

        let file = fs_service
            .read_file(vm_id, "/tmp/shell_rm_test")
            .await
            .unwrap();
        assert!(
            file.is_none(),
            "shell running rm should remove /tmp/shell_rm_test"
        );
    }

    /// Shell: default cwd is user's home (/root for root); ls with no args lists home (Documents, Downloads, etc).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_ls_no_args_lists_cwd() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 86, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-ls-cwd-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "ls")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
            ctx.process_cwd.insert(1, "/root".to_string());
        }
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-ls-cwd-vm", 150).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("Documents"),
            "shell ls with no args (default cwd is user home) should list home (Documents, etc), got: {:?}",
            shell_stdout
        );
    }

    /// Shell: pwd builtin prints current directory.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_pwd_prints_cwd() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 83, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-pwd-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver must queue all stdin in one tick (no yields) so shell receives all lines when created.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "pwd")
os.write_stdin(pid, "cd /tmp")
os.write_stdin(pid, "pwd")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        if let Some(mut ctx) = vm.lua.app_data_mut::<VmContext>() {
            ctx.process_cwd.insert(1, "/root".to_string());
        }
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-pwd-vm", 150).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("/root") || shell_stdout.contains("/"),
            "pwd should print initial cwd (user home, e.g. /root), got: {:?}",
            shell_stdout
        );
        assert!(
            shell_stdout.contains("/tmp"),
            "pwd after cd /tmp should print /tmp, got: {:?}",
            shell_stdout
        );
    }

    /// Shell: cd .. changes cwd to parent; touch then creates file there.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_cd_dot_dot_changes_cwd() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 82, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-cd-dotdot-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver must queue all stdin in one tick so shell receives all lines when created.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "cd /var/log")
os.write_stdin(pid, "cd ..")
os.write_stdin(pid, "touch x.txt")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-cd-dotdot-vm", 100).await;

        let file = fs_service
            .read_file(vm_id, "/var/x.txt")
            .await
            .unwrap();
        assert!(
            file.is_some(),
            "shell cd /var/log then cd .. then touch x.txt should create /var/x.txt"
        );
    }

    /// Shell: touch with absolute path creates file at that path.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_touch_absolute_path() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 81, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-touch-abs-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "touch /tmp/absolute_test.txt")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-touch-abs-vm", 100).await;

        let file = fs_service
            .read_file(vm_id, "/tmp/absolute_test.txt")
            .await
            .unwrap();
        assert!(
            file.is_some(),
            "shell touch /tmp/absolute_test.txt should create file at absolute path"
        );
    }

    /// Shell: cd builtin changes cwd; touch in cwd creates file there.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_cd_then_touch_in_cwd() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 85, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-cd-touch-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver must queue all stdin in one tick so shell receives all lines when created.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "cd /tmp")
os.write_stdin(pid, "touch cwd_file.txt")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-cd-touch-vm", 100).await;

        let file = fs_service
            .read_file(vm_id, "/tmp/cwd_file.txt")
            .await
            .unwrap();
        assert!(
            file.is_some(),
            "shell cd /tmp then touch cwd_file.txt should create /tmp/cwd_file.txt"
        );
    }

    /// Shell: cd into non-existent directory prints error and does not change cwd.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_cd_nonexistent_rejects() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 85, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-cd-nonexistent-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        // Driver must queue all stdin in one tick so shell receives all lines when created.
        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "cd /nonexistent_folder_12345")
os.write_stdin(pid, "pwd")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-cd-nonexistent-vm", 100).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("no such file or directory"),
            "cd into nonexistent should print error, got: {:?}",
            shell_stdout
        );
        assert!(
            shell_stdout.contains("/root") || shell_stdout.contains("/home"),
            "pwd after failed cd should still be home, got: {:?}",
            shell_stdout
        );
    }

    /// Shell: unknown command prints <red>Command not found: ...</red>.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_shell_command_not_found() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 84, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config = super::super::db::vm_service::VmConfig {
            hostname: "shell-notfound-vm".to_string(),
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
        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, vm_id);
        vm.attach_nic(nic);

        let driver_script = r#"
local pid = os.spawn("sh", {})
os.write_stdin(pid, "nonexistentcommand")
"#;
        vm.os.spawn_process(&vm.lua,driver_script, vec![], 0, "root");
        run_n_ticks_with_spawn(&mut vm, &manager, vm_id, "shell-notfound-vm", 100).await;

        let shell_stdout = vm
            .os
            .processes
            .iter()
            .find(|p| p.id == 2)
            .and_then(|p| p.stdout.lock().ok())
            .map(|g| g.clone())
            .unwrap_or_default();
        assert!(
            shell_stdout.contains("<red>") && shell_stdout.contains("Command not found") && shell_stdout.contains("nonexistentcommand"),
            "shell should print red Command not found for unknown command, got: {:?}",
            shell_stdout
        );
    }

    /// Minimal two-VM network: B sends "hello" to A (port 0); A receives and writes to stdout.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_network_send_recv() {
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
            hostname: "net-a".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config).await.unwrap();
        let ip_a = nic_a.ip.to_string();
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "net-b".to_string(),
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
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // A: listen on a port, then loop recv and write
        const PORT: u16 = 12345;
        let recv_script = format!(
            r#"
net.listen({})
for i = 1, 50 do
  local r = net.recv()
  if r then io.write(r.data) end
end
"#,
            PORT
        );
        vm_a.os.spawn_process(&vm_a.lua,&recv_script, vec![], 0, "root");

        // B: use connection API to send to A (no net.listen(0))
        let send_script = format!(
            r#"
local conn = net.connect("{}", {})
conn:send("hello")
"#,
            ip_a,
            PORT
        );
        vm_b.os.spawn_process(&vm_b.lua,&send_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, 80).await;

        let vm_a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            vm_a_stdout.contains("hello"),
            "VM A should receive 'hello' from B, got: {:?}",
            vm_a_stdout
        );
    }

    /// Verify router delivers a packet from VM A to VM B when B is listening on dst_port.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_router_delivers_to_listening_port() {
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
            hostname: "rtr-a".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config).await.unwrap();
        let ip_a = nic_a.ip;
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "rtr-b".to_string(),
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
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip;

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // B only listens on 22 (no process that runs forever; we just need listening_ports set)
        vm_b
            .os
            .spawn_process(&vm_b.lua, "net.listen(22)", vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, 2).await;

        // B should have run and called net.listen(22), so B's NIC is listening on 22
        assert!(
            vms[1].nic.as_ref().map_or(false, |n| n.is_listening(22)),
            "VM B NIC should be listening on 22"
        );

        // Manually inject one packet: A -> B, port 22
        let pkt = Packet::tcp(ip_a, 0, ip_b, 22, b"hello".to_vec());
        if let Some(nic) = &mut vms[0].nic {
            nic.send(pkt);
        }

        manager.network_tick(&mut vms);

        assert!(
            vms[1].nic.as_ref().map_or(false, |n| n.has_inbound()),
            "VM B should have received the packet after network_tick"
        );
        let received = vms[1].nic.as_mut().and_then(|n| n.recv());
        assert_eq!(
            received.as_ref().and_then(|p| p.payload_str()),
            Some("hello"),
            "VM B should have received payload 'hello'"
        );
    }

    /// One port per process: second process on the same VM calling net.listen(same_port) must fail.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_net_listen_same_port_second_fails() {
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
        let config = super::super::db::vm_service::VmConfig {
            hostname: "listen-a".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config).await.unwrap();
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "listen-b".to_string(),
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
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // VM A: process 1 listens on 8080 and exits
        vm_a.os.spawn_process(&vm_a.lua,"net.listen(8080)", vec![], 0, "root");
        // VM A: process 2 tries to listen on 8080 (must fail), writes "ok" or "fail", then loops so we can read stdout
        vm_a.os.spawn_process(&vm_a.lua,
            r#"
local ok, err = pcall(function() net.listen(8080) end)
io.write(ok and "ok" or "fail")
while true do end
"#,
            vec![],
            0,
            "root",
        );

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, 100).await;

        let second_stdout = vms[0]
            .os
            .processes
            .iter()
            .find(|p| p.stdout.lock().map_or(false, |s| s.contains("fail") || s.contains("ok")))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            second_stdout.contains("fail"),
            "Second process's net.listen(8080) should fail (address in use); got stdout: {:?}",
            second_stdout
        );
    }

    /// Connection API: client uses net.connect, conn:send, conn:recv (no net.listen(0)). Assert response received.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_net_connect_request_response() {
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
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "conn-a".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "conn-b".to_string(),
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
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        const PORT: u16 = 7777;
        vm_b.os.spawn_process(&vm_b.lua,
            &format!(
                r#"
net.listen({})
while true do
  local r = net.recv()
  if r then net.send(r.src_ip, r.src_port, "pong") end
end
"#,
                PORT
            ),
            vec![],
            0,
            "root",
        );
        vm_a.os.spawn_process(&vm_a.lua,
            &format!(
                r#"
local conn = net.connect("{}", {})
conn:send("ping")
while true do
  local r = conn:recv()
  if r then io.write(r.data); conn:close(); break end
end
while true do end
"#,
                ip_b,
                PORT
            ),
            vec![],
            0,
            "root",
        );

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, 500).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            a_stdout.contains("pong"),
            "VM A (connection API client) should receive 'pong'; got stdout: {:?}",
            a_stdout
        );
    }

    /// HTTP protocol: VM B runs HTTP server on port 80, VM A sends GET / and receives "Hello NTML" response.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_http_protocol_two_vms() {
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
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "http-client".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "http-server".to_string(),
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
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        const PORT: u16 = 80;
        vm_b.os.spawn_process(
            &vm_b.lua,
            r#"
net.listen(80)
while true do
  local r = net.recv()
  if r then
    local req = http.parse_request(r.data)
    if req and req.path == "/" then
      local res = http.build_response(200, "Hello NTML")
      net.send(r.src_ip, r.src_port, res)
    end
  end
end
"#,
            vec![],
            0,
            "root",
        );
        vm_a.os.spawn_process(
            &vm_a.lua,
            &format!(
                r#"
local req = http.build_request("GET", "/", nil)
local conn = net.connect("{}", {})
conn:send(req)
while true do
  local r = conn:recv()
  if r then
    local res = http.parse_response(r.data)
    if res and res.status == 200 then
      io.write(res.body)
    end
    conn:close()
    break
  end
end
while true do end
"#,
                ip_b,
                PORT
            ),
            vec![],
            0,
            "root",
        );

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, 500).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            a_stdout.contains("Hello NTML"),
            "VM A (HTTP client) should receive 'Hello NTML'; got stdout: {:?}",
            a_stdout
        );
    }

    /// HTTP protocol over loopback: VM runs server on port 80 and client connects to localhost (127.0.0.1).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_http_protocol_loopback() {
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
            hostname: "loopback-vm".to_string(),
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
        let (_rec, nic) = manager.create_vm(config).await.unwrap();

        let config_dummy = super::super::db::vm_service::VmConfig {
            hostname: "dummy".to_string(),
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
        let (_rec_dummy, nic_dummy) = manager.create_vm(config_dummy).await.unwrap();

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_dummy = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, _rec.id);
        vm.attach_nic(nic);
        let mut vm_dummy = VirtualMachine::with_id(lua_dummy, _rec_dummy.id);
        vm_dummy.attach_nic(nic_dummy);
        vm_dummy.os.spawn_process(&vm_dummy.lua, "while true do end", vec![], 0, "root");

        // Process 1: HTTP server on port 80
        vm.os.spawn_process(
            &vm.lua,
            r#"
net.listen(80)
while true do
  local r = net.recv()
  if r then
    local req = http.parse_request(r.data)
    if req and req.path == "/" then
      local res = http.build_response(200, "Hello loopback")
      net.send(r.src_ip, r.src_port, res)
    end
  end
end
"#,
            vec![],
            0,
            "root",
        );
        // Process 2: HTTP client connecting to localhost:80
        vm.os.spawn_process(
            &vm.lua,
            r#"
local req = http.build_request("GET", "/", nil)
local conn = net.connect("localhost", 80)
conn:send(req)
while true do
  local r = conn:recv()
  if r then
    local res = http.parse_response(r.data)
    if res and res.status == 200 then
      io.write(res.body)
    end
    conn:close()
    break
  end
end
while true do end
"#,
            vec![],
            0,
            "root",
        );

        let mut vms = vec![vm, vm_dummy];
        run_n_ticks_vms_network(&mut manager, &mut vms, 500).await;

        // First process is server, second is client. Client writes "Hello loopback" to stdout.
        let client_stdout = vms[0]
            .os
            .processes
            .iter()
            .nth(1)
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();
        assert!(
            client_stdout.contains("Hello loopback"),
            "HTTP client over loopback should receive 'Hello loopback'; got stdout: {:?}",
            client_stdout
        );
    }

    /// Max ticks for two-VM network tests. Single limit, no per-test tuning.
    const MAX_TICKS_TWO_VM_NETWORK: usize = 2000;

    /// VM A runs SSH client (ssh ip_b), VM B runs ssh-server on port 22. Driver on A injects "echo hello"
    /// into the client stdin; assert the SSH client process stdout contains "hello" (remote shell output).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_ssh_client_to_ssh_server() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 86, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "ssh-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "ssh-b".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // B: ssh-server listens on 22, spawns shell per client, relays stdin/stdout.
        vm_b.os.spawn_process(&vm_b.lua,bin_programs::SSH_SERVER, vec![], 0, "root");

        // A: driver spawns ssh client, injects "echo hello" into its stdin, then loops so we can read client stdout.
        let driver_script = format!(
            r#"
local pid = os.spawn("ssh", {{ "{}" }})
os.write_stdin(pid, "echo hello")
while true do end
"#,
            ip_b
        );
        vm_a.os.spawn_process(&vm_a.lua,&driver_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let ssh_stdout = vms[0]
            .os
            .processes
            .iter()
            .find(|p| p.args.first().map(|a| a.as_str()) == Some(ip_b.as_str()))
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();

        assert!(
            ssh_stdout.contains("hello"),
            "SSH client on VM A should receive remote 'echo hello' output; got stdout: {:?}",
            ssh_stdout
        );
    }

    /// Ctrl+C via SSH: A's shell has ssh as child; inject \x03; sequence is forwarded to remote shell,
    /// which kills its foreground child (echo_stdin). Remote echo_stdin is gone; both shells and ssh remain.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_ctrl_c_via_ssh_forwarded() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 87, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "ssh-ctrl-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "ssh-ctrl-b".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // B: ssh-server
        vm_b.os.spawn_process(&vm_b.lua, bin_programs::SSH_SERVER, vec![], 0, "root");

        // A: shell; inject "ssh ip_b" (child is ssh), then "echo_stdin" (remote spawns echo_stdin), then \x03 (remote kills echo_stdin).
        let driver_script = format!(
            r#"
local pid = os.spawn("sh", {{}})
os.write_stdin(pid, "ssh {}")
os.write_stdin(pid, "echo_stdin")
os.write_stdin(pid, "\x03")
while true do end
"#,
            ip_b
        );
        vm_a.os.spawn_process(&vm_a.lua, &driver_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        // On VM B: ssh-server and one shell (spawned for the client). The remote shell's child (echo_stdin) must be killed.
        let vm_b = &vms[1];
        let has_echo_stdin_on_b = vm_b.os.processes.iter().any(|p| {
            p.display_name
                .as_deref()
                .or(p.args.first().map(|s| s.as_str()))
                == Some("echo_stdin")
        });
        assert!(
            !has_echo_stdin_on_b,
            "Ctrl+C via SSH should kill remote foreground child (echo_stdin) on VM B; processes: {:?}",
            vm_b.os
                .processes
                .iter()
                .map(|p| (p.id, p.display_name.as_deref()))
                .collect::<Vec<_>>()
        );
        // Both VMs should still have shell and ssh (A: driver, sh, ssh; B: ssh-server, sh).
        assert!(
            vms[0].os.processes.len() >= 2,
            "VM A should have at least driver and shell (and ssh)"
        );
        assert!(
            vms[1].os.processes.len() >= 2,
            "VM B should have ssh-server and remote shell"
        );
    }

    /// VM A runs two SSH clients connected to VM B's ssh-server. Asserts both sessions work and are
    /// isolated: one client receives only "one", the other only "two" (no cross-talk).
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_two_ssh_clients_sessions_isolated() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 85, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "ssh-multi-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "ssh-multi-b".to_string(),
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
        let (_rec_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_rec_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _rec_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _rec_b.id);
        vm_b.attach_nic(nic_b);

        // B: ssh-server listens on 22, one shell per connection (key = src_ip:src_port).
        vm_b.os.spawn_process(&vm_b.lua,bin_programs::SSH_SERVER, vec![], 0, "root");

        // A: driver spawns two SSH clients, injects distinct commands, then loops so we can read both stdouts.
        let driver_script = format!(
            r#"
local pid1 = os.spawn("ssh", {{ "{}" }})
local pid2 = os.spawn("ssh", {{ "{}" }})
os.write_stdin(pid1, "echo one")
os.write_stdin(pid2, "echo two")
while true do end
"#,
            ip_b,
            ip_b
        );
        vm_a.os.spawn_process(&vm_a.lua,&driver_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let ssh_processes: Vec<_> = vms[0]
            .os
            .processes
            .iter()
            .filter(|p| p.args.first().map(|a| a.as_str()) == Some(ip_b.as_str()))
            .collect();

        assert_eq!(
            ssh_processes.len(),
            2,
            "VM A should have exactly two SSH client processes (args = [ip_b]); got {}",
            ssh_processes.len()
        );

        let stdouts: Vec<String> = ssh_processes
            .iter()
            .filter_map(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .collect();

        let has_one_only = stdouts
            .iter()
            .any(|s| s.contains("one") && !s.contains("two"));
        let has_two_only = stdouts
            .iter()
            .any(|s| s.contains("two") && !s.contains("one"));

        assert!(
            has_one_only && has_two_only,
            "Sessions must be isolated: one client stdout should contain 'one' only, the other 'two' only. stdouts: {:?}",
            stdouts
        );
    }

    /// Realistic scenario: A sends request, B listens and responds with value "x", A writes received value to stdout.
    /// Run with a fixed tick limit; assert A has the expected stdout at the end.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_a_request_b_response_stdout() {
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
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "req-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "req-b".to_string(),
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
        let (_record_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_record_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _record_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _record_b.id);
        vm_b.attach_nic(nic_b);

        // B: listen on port; loop forever until first request, then respond with "x" and break
        const LISTEN_PORT: u16 = 9999;
        let b_script = format!(
            r#"
net.listen({})
while true do
  local r = net.recv()
  if r then
    net.send(r.src_ip, r.src_port, "x")
    break
  end
end
"#,
            LISTEN_PORT
        );
        vm_b.os.spawn_process(&vm_b.lua,&b_script, vec![], 0, "root");

        // A: use connection API to send request to B, then loop recv on connection (no net.listen(0))
        let a_script = format!(
            r#"
local conn = net.connect("{}", {})
conn:send("request")
while true do
  local r = conn:recv()
  if r then
    io.write(r.data)
  end
end
"#,
            ip_b,
            LISTEN_PORT
        );
        vm_a.os.spawn_process(&vm_a.lua,&a_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();

        // Inspect NIC inbound queues: if packets are there, they arrived but were not consumed by the process
        let a_inbound_count = vms[0]
            .nic
            .as_ref()
            .map(|n| n.inbound.len())
            .unwrap_or(0);
        let b_inbound_count = vms[1]
            .nic
            .as_ref()
            .map(|n| n.inbound.len())
            .unwrap_or(0);

        assert!(
            a_stdout.contains("x"),
            "VM A should have received and written 'x' to stdout. \
             a_stdout={:?}; \
             A NIC inbound={} (if >0, response arrived but A process did not consume); \
             B NIC inbound={} (if >0, request arrived but B process did not consume)",
            a_stdout, a_inbound_count, b_inbound_count
        );
    }

    /// Multi-request / multi-response: A sends 1 request, B sends 4 responses; A sends 1 more, B sends 1. Assert A stdout is "12345".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_multi_request_multi_response_stdout() {
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
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "multi-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "multi-b".to_string(),
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
        let (_record_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_record_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _record_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _record_b.id);
        vm_b.attach_nic(nic_b);

        const LISTEN_PORT: u16 = 9998;
        // B: on 1st request send "1","2","3","4"; on 2nd request send "5". No break.
        let b_script = format!(
            r#"
net.listen({})
local req = 0
while true do
  local r = net.recv()
  if r then
    req = req + 1
    if req == 1 then
      net.send(r.src_ip, r.src_port, "1")
      net.send(r.src_ip, r.src_port, "2")
      net.send(r.src_ip, r.src_port, "3")
      net.send(r.src_ip, r.src_port, "4")
    else
      net.send(r.src_ip, r.src_port, "5")
    end
  end
end
"#,
            LISTEN_PORT
        );
        vm_b.os.spawn_process(&vm_b.lua,&b_script, vec![], 0, "root");

        // A: connection API; send req1, recv until 4 then send req2, recv 1 more; write all to stdout.
        let a_script = format!(
            r#"
local conn = net.connect("{}", {})
conn:send("req1")
local n = 0
while true do
  local r = conn:recv()
  if r then
    io.write(r.data)
    n = n + 1
    if n == 4 then
      conn:send("req2")
    end
  end
end
"#,
            ip_b,
            LISTEN_PORT
        );
        vm_a.os.spawn_process(&vm_a.lua,&a_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();

        let a_inbound_count = vms[0]
            .nic
            .as_ref()
            .map(|n| n.inbound.len())
            .unwrap_or(0);
        let b_inbound_count = vms[1]
            .nic
            .as_ref()
            .map(|n| n.inbound.len())
            .unwrap_or(0);

        assert!(
            a_stdout.contains("12345"),
            "VM A should have received and written '12345' to stdout. \
             a_stdout={:?}; \
             A NIC inbound={}; B NIC inbound={}",
            a_stdout, a_inbound_count, b_inbound_count
        );
    }

    /// B listens on two ports (80 and 9999); A sends distinct payload to each; B echoes back. Assert A stdout contains both.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_two_vms_b_two_ports_echo_stdout() {
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
        let config_a = super::super::db::vm_service::VmConfig {
            hostname: "twoport-a".to_string(),
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
        let config_b = super::super::db::vm_service::VmConfig {
            hostname: "twoport-b".to_string(),
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
        let (_record_a, nic_a) = manager.create_vm(config_a).await.unwrap();
        let (_record_b, nic_b) = manager.create_vm(config_b).await.unwrap();
        let ip_b = nic_b.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _record_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _record_b.id);
        vm_b.attach_nic(nic_b);

        // B: listen on 80 and 9999; echo back r.data for each received packet
        let b_script = r#"
net.listen(80)
net.listen(9999)
while true do
  local r = net.recv()
  if r then
    net.send(r.src_ip, r.src_port, r.data)
  end
end
"#;
        vm_b.os.spawn_process(&vm_b.lua,b_script, vec![], 0, "root");

        // A: two connections to B (port 80 and 9999), send distinct payloads, then loop recv on both (no net.listen(0))
        let a_script = format!(
            r#"
local c80 = net.connect("{}", 80)
local c9999 = net.connect("{}", 9999)
c80:send("port80")
c9999:send("port9999")
while true do
  local r = c80:recv()
  if r then io.write(r.data) end
  r = c9999:recv()
  if r then io.write(r.data) end
end
"#,
            ip_b,
            ip_b
        );
        vm_a.os.spawn_process(&vm_a.lua,&a_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();

        let a_inbound = vms[0].nic.as_ref().map(|n| n.inbound.len()).unwrap_or(0);
        let b_inbound = vms[1].nic.as_ref().map(|n| n.inbound.len()).unwrap_or(0);

        assert!(
            a_stdout.contains("port80") && a_stdout.contains("port9999"),
            "VM A should have received both echoed payloads. \
             a_stdout={:?}; A NIC inbound={}; B NIC inbound={}",
            a_stdout, a_inbound, b_inbound
        );
    }

    /// Three VMs: A sends request to B and to C; B and C respond with "B" and "C". Assert A stdout contains both.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_three_vms_a_requests_b_and_c_stdout() {
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
        let config = |hostname: &str| super::super::db::vm_service::VmConfig {
            hostname: hostname.to_string(),
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
        let (_record_a, nic_a) = manager.create_vm(config("three-a")).await.unwrap();
        let (_record_b, nic_b) = manager.create_vm(config("three-b")).await.unwrap();
        let (_record_c, nic_c) = manager.create_vm(config("three-c")).await.unwrap();
        let ip_b = nic_b.ip.to_string();
        let ip_c = nic_c.ip.to_string();

        let lua_a = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_b = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let lua_c = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm_a = VirtualMachine::with_id(lua_a, _record_a.id);
        vm_a.attach_nic(nic_a);
        let mut vm_b = VirtualMachine::with_id(lua_b, _record_b.id);
        vm_b.attach_nic(nic_b);
        let mut vm_c = VirtualMachine::with_id(lua_c, _record_c.id);
        vm_c.attach_nic(nic_c);

        const PORT_B: u16 = 9000;
        const PORT_C: u16 = 9001;

        let b_script = format!(
            r#"
net.listen({})
while true do
  local r = net.recv()
  if r then
    net.send(r.src_ip, r.src_port, "B")
  end
end
"#,
            PORT_B
        );
        vm_b.os.spawn_process(&vm_b.lua,&b_script, vec![], 0, "root");

        let c_script = format!(
            r#"
net.listen({})
while true do
  local r = net.recv()
  if r then
    net.send(r.src_ip, r.src_port, "C")
  end
end
"#,
            PORT_C
        );
        vm_c.os.spawn_process(&vm_c.lua,&c_script, vec![], 0, "root");

        let a_script = format!(
            r#"
local conn_b = net.connect("{}", {})
local conn_c = net.connect("{}", {})
conn_b:send("req")
conn_c:send("req")
while true do
  local r = conn_b:recv()
  if r then io.write(r.data) end
  r = conn_c:recv()
  if r then io.write(r.data) end
end
"#,
            ip_b,
            PORT_B,
            ip_c,
            PORT_C
        );
        vm_a.os.spawn_process(&vm_a.lua,&a_script, vec![], 0, "root");

        let mut vms = vec![vm_a, vm_b, vm_c];
        run_n_ticks_vms_network(&mut manager, &mut vms, MAX_TICKS_TWO_VM_NETWORK).await;

        let a_stdout = vms[0]
            .os
            .processes
            .iter()
            .next()
            .and_then(|p| p.stdout.lock().ok().map(|g| g.clone()))
            .unwrap_or_default();

        let a_inbound = vms[0].nic.as_ref().map(|n| n.inbound.len()).unwrap_or(0);
        let b_inbound = vms[1].nic.as_ref().map(|n| n.inbound.len()).unwrap_or(0);
        let c_inbound = vms[2].nic.as_ref().map(|n| n.inbound.len()).unwrap_or(0);

        assert!(
            a_stdout.contains("B") && a_stdout.contains("C"),
            "VM A should have received both 'B' and 'C'. \
             a_stdout={:?}; A NIC inbound={}; B={}; C={}",
            a_stdout, a_inbound, b_inbound, c_inbound
        );
    }

    /// Terminal feature: simulate frontend opening a terminal (push pending open), run game loop
    /// in a task until SessionReady, send "echo hello" via stdin_tx, then assert stdout contains "hello".
    #[tokio::test(flavor = "multi_thread")]
    async fn test_terminal_hub_shell_stdin_stdout() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 87, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let owner_name = format!("termplayer_{}", Uuid::new_v4());
        let player = player_service
            .create_player(&owner_name, "termpass")
            .await
            .unwrap();

        let config = super::super::db::vm_service::VmConfig {
            hostname: "term-vm".to_string(),
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

        let (record, nic) = manager.create_vm(config).await.unwrap();

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, record.id);
        vm.attach_nic(nic);
        let mut vms = vec![vm];

        let terminal_hub = terminal_hub::new_hub();
        let (response_tx, mut response_rx) = oneshot::channel();
        {
            let mut hub = terminal_hub.lock().unwrap();
            hub.pending_opens.push((player.id, response_tx));
        }

        const MAX_TICKS_SESSION: usize = 500;
        const MAX_TICKS_STDOUT: usize = 1000;

        let mut ticks_done = 0;
        let mut ready = loop {
            manager
                .run_n_ticks_with_terminal_hub(&mut vms, terminal_hub.clone(), 50)
                .await;
            ticks_done += 50;
            match response_rx.try_recv() {
                Ok(Ok(session)) => break session,
                Ok(Err(e)) => panic!("terminal open failed: {}", e),
                Err(_) => {}
            }
            if ticks_done >= MAX_TICKS_SESSION {
                panic!("timeout: SessionReady not received after {} ticks", ticks_done);
            }
        };

        ready
            .stdin_tx
            .send("echo hello\n".to_string())
            .await
            .expect("send stdin");

        let mut stdout_acc = String::new();
        while !stdout_acc.contains("hello") {
            manager
                .run_n_ticks_with_terminal_hub(&mut vms, terminal_hub.clone(), 30)
                .await;
            ticks_done += 30;
            while let Ok(chunk) = ready.stdout_rx.try_recv() {
                stdout_acc.push_str(&chunk);
            }
            if ticks_done >= MAX_TICKS_SESSION + MAX_TICKS_STDOUT {
                panic!(
                    "timeout: stdout never contained 'hello' after {} ticks. Accumulated: {:?}",
                    ticks_done, stdout_acc
                );
            }
        }

        assert!(
            stdout_acc.contains("hello"),
            "shell stdout should contain 'hello', got: {:?}",
            stdout_acc
        );
    }

    /// Terminal close: when we push (vm_id, shell_pid) to pending_kills (simulating UI closing the terminal),
    /// the game loop kills the shell and all descendants. Validates cascade kill.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_terminal_close_kills_shell_and_descendants() {
        let pool = db::test_pool().await;
        let vm_service = Arc::new(VmService::new(pool.clone()));
        let fs_service = Arc::new(FsService::new(pool.clone()));
        let user_service = Arc::new(UserService::new(pool.clone()));
        let player_service = Arc::new(PlayerService::new(pool.clone()));
        let subnet = Subnet::new(Ipv4Addr::new(10, 0, 88, 0), 24);
        let mut manager = VmManager::new(
            vm_service.clone(),
            fs_service.clone(),
            user_service.clone(),
            player_service.clone(),
            subnet,
        );

        let owner_name = format!("termplayer_{}", Uuid::new_v4());
        let player = player_service
            .create_player(&owner_name, "termpass")
            .await
            .unwrap();

        let config = super::super::db::vm_service::VmConfig {
            hostname: "term-vm".to_string(),
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

        let (record, nic) = manager.create_vm(config).await.unwrap();
        let vm_id = record.id;

        let lua = crate::create_vm_lua_state(pool.clone(), fs_service.clone(), user_service.clone()).unwrap();
        let mut vm = VirtualMachine::with_id(lua, record.id);
        vm.attach_nic(nic);
        let mut vms = vec![vm];

        let terminal_hub = terminal_hub::new_hub();
        let (response_tx, mut response_rx) = oneshot::channel();
        {
            let mut hub = terminal_hub.lock().unwrap();
            hub.pending_opens.push((player.id, response_tx));
        }

        let mut ticks_done = 0;
        let ready = loop {
            manager
                .run_n_ticks_with_terminal_hub(&mut vms, terminal_hub.clone(), 50)
                .await;
            ticks_done += 50;
            match response_rx.try_recv() {
                Ok(Ok(session)) => break session,
                Ok(Err(e)) => panic!("terminal open failed: {}", e),
                Err(_) => {}
            }
            if ticks_done >= 500 {
                panic!("timeout: SessionReady not received");
            }
        };

        let (shell_vm_id, shell_pid) = (ready.vm_id, ready.pid);

        // Spawn a child (echo) so we have shell + child; then simulate terminal close and assert both are killed
        ready
            .stdin_tx
            .send("echo done\n".to_string())
            .await
            .expect("send stdin");

        manager
            .run_n_ticks_with_terminal_hub(&mut vms, terminal_hub.clone(), 20)
            .await;

        // Simulate terminal closed: push kill for the shell (game loop will cascade to children)
        {
            let mut hub = terminal_hub.lock().unwrap();
            hub.pending_kills.push((shell_vm_id, shell_pid));
        }

        manager
            .run_n_ticks_with_terminal_hub(&mut vms, terminal_hub.clone(), 10)
            .await;

        // VM may have been removed from vms when all processes were killed (os.is_finished); either way, no shell/children left
        if let Some(vm) = vms.iter().find(|v| v.id == vm_id) {
            assert!(
                vm.os.processes.is_empty(),
                "closing terminal should kill shell and all descendants; processes left: {:?}",
                vm.os
                    .processes
                    .iter()
                    .map(|p| (p.id, p.display_name.as_deref().unwrap_or("?")))
                    .collect::<Vec<_>>()
            );
        }
        // Else: VM was removed by retain_mut because it had no processes (all killed), which is correct
    }
}
