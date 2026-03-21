//! Snapshot of cluster state for admin API. Updated periodically by the game loop.

use super::db::fs_service::FsService;
use super::db::vm_service::VmService;
use super::vm::VirtualMachine;
use super::vm_manager::ActiveVm;
use dashmap::DashMap;
use std::time::Instant;
use uuid::Uuid;

/// Per-VM snapshot for admin ListVms.
#[derive(Debug, Clone)]
pub struct VmSnapshotInfo {
    pub id: Uuid,
    pub hostname: String,
    pub dns_name: Option<String>,
    pub ip: Option<String>,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub cpu_cores: i16,
    pub memory_mb: i32,
    pub disk_mb: i32,
    pub owner_id: Option<Uuid>,
    pub real_memory_bytes: u64,
    pub disk_used_bytes: i64,
    pub ticks_per_second: u32,
    pub remaining_ticks: u32,
}

/// Cluster-wide snapshot for admin API.
#[derive(Debug, Clone)]
pub struct ClusterSnapshot {
    pub vms: Vec<VmSnapshotInfo>,
    pub tick_count: u64,
    pub uptime_secs: f64,
    pub start_instant: Instant,
}

/// Network topology node for React Flow.
#[derive(Debug, Clone)]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub ip: String,
    pub subnet: String,
    pub node_type: String,
}

/// Network topology edge for React Flow.
#[derive(Debug, Clone)]
pub struct TopologyEdge {
    pub source_id: String,
    pub target_id: String,
    pub same_subnet: bool,
}

impl ClusterSnapshot {
    pub fn empty(start_instant: Instant) -> Self {
        Self {
            vms: vec![],
            tick_count: 0,
            uptime_secs: 0.0,
            start_instant,
        }
    }

    /// Effective TPS (ticks per second) since start.
    pub fn effective_tps(&self) -> f64 {
        if self.uptime_secs <= 0.0 {
            return 0.0;
        }
        self.tick_count as f64 / self.uptime_secs
    }

    /// Build network topology: nodes (VMs + routers + subnets) and edges.
    /// Edges: VM→router, VM→subnet, router→subnet. No VM-to-VM edges.
    pub fn build_network_topology(&self) -> (Vec<TopologyNode>, Vec<TopologyEdge>) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut gateway_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut subnet_ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for v in &self.vms {
            let id = v.id.to_string();
            let label = v.dns_name.as_deref().unwrap_or(&v.hostname).to_string();
            let ip = v.ip.clone().unwrap_or_else(|| "?".to_string());
            let subnet = v.subnet.clone().unwrap_or_default();
            let gateway = v.gateway.clone().unwrap_or_default();

            nodes.push(TopologyNode {
                id: id.clone(),
                label,
                ip: ip.clone(),
                subnet: subnet.clone(),
                node_type: "vm".to_string(),
            });

            // VM → router (gateway)
            if !gateway.is_empty() {
                let (gw_id, is_new_router) = if let Some(existing) = gateway_ids.get(&gateway) {
                    (existing.clone(), false)
                } else {
                    let gw_node_id = format!("router-{}", gateway.replace('.', "-"));
                    gateway_ids.insert(gateway.clone(), gw_node_id.clone());
                    nodes.push(TopologyNode {
                        id: gw_node_id.clone(),
                        label: format!("Router {}", gateway),
                        ip: gateway.clone(),
                        subnet: subnet.clone(),
                        node_type: "router".to_string(),
                    });
                    (gw_node_id, true)
                };
                edges.push(TopologyEdge {
                    source_id: id.clone(),
                    target_id: gw_id.clone(),
                    same_subnet: true,
                });

                // Router → subnet (only when we just created the router, to avoid duplicates)
                if is_new_router && !subnet.is_empty() {
                    let subnet_id = if let Some(existing) = subnet_ids.get(&subnet) {
                        existing.clone()
                    } else {
                        let subnet_node_id = format!("subnet-{}", subnet.replace(['.', '/'], "-"));
                        subnet_ids.insert(subnet.clone(), subnet_node_id.clone());
                        nodes.push(TopologyNode {
                            id: subnet_node_id.clone(),
                            label: subnet.clone(),
                            ip: String::new(),
                            subnet: subnet.clone(),
                            node_type: "subnet".to_string(),
                        });
                        subnet_node_id
                    };
                    edges.push(TopologyEdge {
                        source_id: gw_id,
                        target_id: subnet_id,
                        same_subnet: true,
                    });
                }
            }

            // VM → subnet (VM belongs to subnet)
            if !subnet.is_empty() {
                let subnet_id = if let Some(existing) = subnet_ids.get(&subnet) {
                    existing.clone()
                } else {
                    let subnet_node_id = format!("subnet-{}", subnet.replace(['.', '/'], "-"));
                    subnet_ids.insert(subnet.clone(), subnet_node_id.clone());
                    nodes.push(TopologyNode {
                        id: subnet_node_id.clone(),
                        label: subnet.clone(),
                        ip: String::new(),
                        subnet: subnet.clone(),
                        node_type: "subnet".to_string(),
                    });
                    subnet_node_id
                };
                edges.push(TopologyEdge {
                    source_id: id,
                    target_id: subnet_id,
                    same_subnet: true,
                });
            }
        }

        (nodes, edges)
    }
}

/// Build a fresh snapshot from current VM state. Called periodically from the game loop.
pub async fn build_snapshot(
    vms: &[VirtualMachine],
    active_vms: &[ActiveVm],
    vm_lua_memory_store: &DashMap<Uuid, u64>,
    vm_service: &VmService,
    fs_service: &FsService,
    tick_count: u64,
    start_instant: Instant,
) -> ClusterSnapshot {
    let uptime_secs = start_instant.elapsed().as_secs_f64();

    let mut vm_infos = Vec::with_capacity(vms.len());
    let active_map: std::collections::HashMap<Uuid, &ActiveVm> =
        active_vms.iter().map(|a| (a.id, a)).collect();

    for vm in vms {
        let active = active_map.get(&vm.id);
        let (hostname, dns_name, ip_str, subnet_str, gateway_str) = if let Some(a) = active {
            (
                a.hostname.clone(),
                a.dns_name.clone(),
                a.ip.map(|ip| ip.to_string()),
                vm.nic.as_ref().map(|n| n.subnet.to_string()),
                vm.nic.as_ref().map(|n| n.gateway.to_string()),
            )
        } else {
            (
                format!("vm-{}", vm.id),
                None,
                vm.nic.as_ref().map(|n| n.ip.to_string()),
                vm.nic.as_ref().map(|n| n.subnet.to_string()),
                vm.nic.as_ref().map(|n| n.gateway.to_string()),
            )
        };

        let record = vm_service.get_vm(vm.id).await.ok().flatten();
        let (owner_id, disk_mb) = record
            .as_ref()
            .map(|r| (r.owner_id, r.disk_mb))
            .unwrap_or((None, vm.memory_mb as i32));

        let disk_used_bytes = fs_service.disk_usage_bytes(vm.id).await.unwrap_or(0);

        let real_memory_bytes = vm_lua_memory_store
            .get(&vm.id)
            .map(|v| *v)
            .unwrap_or(0);

        vm_infos.push(VmSnapshotInfo {
            id: vm.id,
            hostname,
            dns_name,
            ip: ip_str,
            subnet: subnet_str,
            gateway: gateway_str,
            cpu_cores: vm.cpu_cores,
            memory_mb: vm.memory_mb,
            disk_mb,
            owner_id,
            real_memory_bytes,
            disk_used_bytes,
            ticks_per_second: vm.ticks_per_second,
            remaining_ticks: vm.remaining_ticks,
        });
    }

    ClusterSnapshot {
        vms: vm_infos,
        tick_count,
        uptime_secs,
        start_instant,
    }
}
