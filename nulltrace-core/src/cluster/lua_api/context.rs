#![allow(dead_code)]

use crate::net::ip::Ipv4Addr;
use crate::net::packet::Packet;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
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

    /// stdin/stdout for the currently executing process (set before each tick)
    pub current_stdin: Option<Arc<Mutex<VecDeque<String>>>>,
    pub current_stdout: Option<Arc<Mutex<String>>>,

    /// Args for the currently executing process (set before each tick)
    pub process_args: Vec<String>,

    /// Queue of (program_name, args, uid, username) to spawn after current tick (from os.exec)
    pub spawn_queue: Vec<(String, Vec<String>, i32, String)>,
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
            current_stdin: None,
            current_stdout: None,
            process_args: Vec::new(),
            spawn_queue: Vec::new(),
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
        self.current_stdin = None;
        self.current_stdout = None;
        self.process_args.clear();
        self.spawn_queue.clear();
    }

    /// Set the current process's I/O and args before tick.
    pub fn set_current_process(
        &mut self,
        stdin: Arc<Mutex<VecDeque<String>>>,
        stdout: Arc<Mutex<String>>,
        args: Vec<String>,
    ) {
        self.current_stdin = Some(stdin);
        self.current_stdout = Some(stdout);
        self.process_args = args;
    }
}
