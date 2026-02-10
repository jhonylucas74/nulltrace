#![allow(dead_code)]

use super::ip::Ipv4Addr;
use super::packet::Packet;
use super::router::Router;
use std::collections::HashMap;
use uuid::Uuid;

/// The NetManager bridges local cluster networking with the global game world.
///
/// It runs on each K8s pod and:
/// - Manages all routers on the pod
/// - Tracks which VMs (by IP) are local
/// - Sends cross-pod packets via Redis Pub/Sub
/// - Receives incoming packets from other pods
///
/// Redis channels:
///   net:pod:{cluster_id}     — incoming packets for this pod
///   net:broadcast            — global announcements
///
/// Redis keys:
///   net:route:{subnet}       — which cluster_id owns this subnet
///   net:arp:{ip}             — cluster_id:vm_uuid
///   net:pods                 — SET of all active cluster_ids
///
/// TODO: Implement Redis Pub/Sub in Phase 4.
pub struct NetManager {
    pub cluster_id: String,
    pub routers: Vec<Router>,
    pub vm_registry: HashMap<Ipv4Addr, Uuid>,
    // Will hold redis::Client when Phase 4 is implemented
}

impl NetManager {
    pub fn new(cluster_id: String) -> Self {
        Self {
            cluster_id,
            routers: Vec::new(),
            vm_registry: HashMap::new(),
        }
    }

    /// Register a VM's IP as local to this pod.
    pub fn register_vm(&mut self, ip: Ipv4Addr, vm_id: Uuid) {
        self.vm_registry.insert(ip, vm_id);
    }

    /// Unregister a VM (e.g., when it shuts down).
    pub fn unregister_vm(&mut self, ip: &Ipv4Addr) {
        self.vm_registry.remove(ip);
    }

    /// Check if an IP belongs to a VM on this pod.
    pub fn is_local(&self, ip: &Ipv4Addr) -> bool {
        self.vm_registry.contains_key(ip)
    }

    /// Add a router to this pod's network.
    pub fn add_router(&mut self, router: Router) {
        self.routers.push(router);
    }

    /// Send a packet to another pod via Redis Pub/Sub.
    /// Redis channel: `net:pod:{dst_cluster_id}`
    pub fn send_cross_pod(&self, _packet: Packet, _dst_cluster: &str) {
        // TODO: Phase 4 — PUBLISH to Redis
    }

    /// Announce this pod's subnets to the global routing table.
    /// Redis key: `net:route:{subnet}` = cluster_id
    pub fn announce_subnets(&self) {
        // TODO: Phase 4 — write to Redis
    }

    /// Start listening for incoming packets.
    /// Redis SUBSCRIBE: `net:pod:{cluster_id}`
    pub fn start_listening(&self) {
        // TODO: Phase 4 — Redis Pub/Sub subscriber loop
    }

    /// Handle an incoming cross-pod packet.
    fn handle_inbound(&self, _packet: Packet) {
        // TODO: Phase 4 — route to the correct local VM
    }

    /// Lookup which pod owns a destination IP.
    /// Redis key: `net:arp:{ip}`
    pub fn lookup_pod(&self, _dst_ip: Ipv4Addr) -> Option<String> {
        // TODO: Phase 4 — query Redis
        None
    }
}
