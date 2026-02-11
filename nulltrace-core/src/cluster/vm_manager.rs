#![allow(dead_code)]

use super::db::fs_service::FsService;
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
use tokio::time::{sleep_until, Instant as TokioInstant};
use uuid::Uuid;

const TPS: u32 = 60;
const TICK_TIME: Duration = Duration::from_millis(1000 / TPS as u64);

pub struct VmManager {
    pub vm_service: Arc<VmService>,
    pub fs_service: Arc<FsService>,
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
    pub ip: Option<Ipv4Addr>,
}

impl VmManager {
    pub fn new(
        vm_service: Arc<VmService>,
        fs_service: Arc<FsService>,
        subnet: Subnet,
    ) -> Self {
        let mut router = Router::new();
        router.add_interface(subnet.gateway(), subnet.clone());

        Self {
            vm_service,
            fs_service,
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

        // Register in DNS
        self.dns.register_a(&record.hostname, nic.ip);

        // Register in NetManager
        self.net_manager.register_vm(nic.ip, id);

        // Track in memory
        self.active_vms.push(ActiveVm {
            id,
            hostname: record.hostname.clone(),
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
            self.dns.unregister_a(&vm.hostname);
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
            self.dns.unregister_a(&vm.hostname);
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

            // Re-register in DNS
            if let Some(ip) = ip {
                self.dns.register_a(&record.hostname, ip);
                self.net_manager.register_vm(ip, record.id);
            }

            self.active_vms.push(ActiveVm {
                id: record.id,
                hostname: record.hostname.clone(),
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
                    }
                    if !process.is_finished() {
                        process.tick();
                    }
                }
                vm.os.processes.retain(|p| !p.is_finished());

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
