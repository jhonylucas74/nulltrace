#![allow(dead_code)]

use super::ip::Ipv4Addr;
use super::packet::Packet;
use super::router::Router;
use redis::Commands;
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
///   dns:a:{hostname}         — IP address (DNS cache)
///   dns:ptr:{ip}             — hostname (reverse DNS cache)
pub struct NetManager {
    pub cluster_id: String,
    pub routers: Vec<Router>,
    pub vm_registry: HashMap<Ipv4Addr, Uuid>,
    redis: Option<redis::Client>,
}

impl NetManager {
    pub fn new(cluster_id: String) -> Self {
        Self {
            cluster_id,
            routers: Vec::new(),
            vm_registry: HashMap::new(),
            redis: None,
        }
    }

    /// Connect to Redis.
    pub fn connect_redis(&mut self, redis_url: &str) -> Result<(), redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        // Test connection
        let mut conn = client.get_connection()?;
        redis::cmd("PING").query::<String>(&mut conn)?;
        self.redis = Some(client);
        Ok(())
    }

    /// Register a VM's IP as local to this pod.
    pub fn register_vm(&mut self, ip: Ipv4Addr, vm_id: Uuid) {
        self.vm_registry.insert(ip, vm_id);

        // Write to Redis ARP table
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let key = format!("net:arp:{}", ip);
                let val = format!("{}:{}", self.cluster_id, vm_id);
                let _: Result<(), _> = conn.set(&key, &val);
            }
        }
    }

    /// Unregister a VM (e.g., when it shuts down).
    pub fn unregister_vm(&mut self, ip: &Ipv4Addr) {
        self.vm_registry.remove(ip);

        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let key = format!("net:arp:{}", ip);
                let _: Result<(), _> = conn.del(&key);
            }
        }
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
    pub fn send_cross_pod(&self, packet: Packet, dst_cluster: &str) {
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let channel = format!("net:pod:{}", dst_cluster);
                let payload = format!(
                    "{}:{}:{}:{}:{}",
                    packet.src_ip, packet.src_port, packet.dst_ip, packet.dst_port,
                    String::from_utf8_lossy(&packet.payload),
                );
                let _: Result<(), _> = conn.publish(&channel, &payload);
            }
        }
    }

    /// Announce this pod's subnets to the global routing table.
    pub fn announce_subnets(&self) {
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                // Register this pod as active
                let _: Result<(), _> = conn.sadd("net:pods", &self.cluster_id);

                // Announce each router's subnets
                for router in &self.routers {
                    for iface in &router.interfaces {
                        let key = format!("net:route:{}", iface.subnet);
                        let _: Result<(), _> = conn.set(&key, &self.cluster_id);
                    }
                }
            }
        }
    }

    /// Lookup which pod owns a destination IP.
    pub fn lookup_pod(&self, dst_ip: Ipv4Addr) -> Option<String> {
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let key = format!("net:arp:{}", dst_ip);
                if let Ok(val) = conn.get::<_, String>(&key) {
                    // val = "cluster_id:vm_uuid"
                    return val.split(':').next().map(|s| s.to_string());
                }
            }
        }
        None
    }

    /// Cache a DNS A record in Redis.
    pub fn cache_dns_a(&self, hostname: &str, ip: Ipv4Addr, ttl_secs: usize) {
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let key = format!("dns:a:{}", hostname);
                let _: Result<(), _> = conn.set_ex(&key, ip.to_string(), ttl_secs);
                let ptr_key = format!("dns:ptr:{}", ip);
                let _: Result<(), _> = conn.set_ex(&ptr_key, hostname, ttl_secs);
            }
        }
    }

    /// Lookup a DNS A record from Redis cache.
    pub fn lookup_dns_a(&self, hostname: &str) -> Option<Ipv4Addr> {
        if let Some(client) = &self.redis {
            if let Ok(mut conn) = client.get_connection() {
                let key = format!("dns:a:{}", hostname);
                if let Ok(val) = conn.get::<_, String>(&key) {
                    return Ipv4Addr::parse(&val);
                }
            }
        }
        None
    }

    /// Check if Redis is connected.
    pub fn is_redis_connected(&self) -> bool {
        self.redis.is_some()
    }
}
