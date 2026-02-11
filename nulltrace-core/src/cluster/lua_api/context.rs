#![allow(dead_code)]

use crate::net::ip::Ipv4Addr;
use crate::net::packet::Packet;
use sqlx::PgPool;
use std::collections::VecDeque;
use uuid::Uuid;

/// Shared context set via `lua.set_app_data()` before each VM tick.
/// Since the game loop ticks one VM at a time, this is safe.
pub struct VmContext {
    pub vm_id: Uuid,
    pub hostname: String,
    pub ip: Option<Ipv4Addr>,
    pub current_pid: u64,
    pub current_uid: i32,
    pub current_username: String,
    pub pool: PgPool,

    // Network I/O buffers — Lua reads/writes these, Rust syncs with NIC after tick
    pub net_outbound: Vec<Packet>,
    pub net_inbound: VecDeque<Packet>,
    pub listening_ports: Vec<u16>,
}

impl VmContext {
    pub fn new(pool: PgPool) -> Self {
        Self {
            vm_id: Uuid::nil(),
            hostname: String::new(),
            ip: None,
            current_pid: 0,
            current_uid: 0,
            current_username: String::from("root"),
            pool,
            net_outbound: Vec::new(),
            net_inbound: VecDeque::new(),
            listening_ports: Vec::new(),
        }
    }

    /// Prepare context for a specific VM's tick.
    pub fn set_vm(&mut self, vm_id: Uuid, hostname: &str, ip: Option<Ipv4Addr>) {
        self.vm_id = vm_id;
        self.hostname = hostname.to_string();
        self.ip = ip;
        self.current_pid = 0;
        self.current_uid = 0;
        self.current_username = String::from("root");
        self.net_outbound.clear();
        self.net_inbound.clear();
        self.listening_ports.clear();
    }
}
