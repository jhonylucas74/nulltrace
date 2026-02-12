#![allow(dead_code)]

use crate::net::ip::Ipv4Addr;
use crate::net::packet::Packet;
use sqlx::PgPool;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Spawn target: from /bin by name or from a file path.
#[derive(Clone, Debug)]
pub enum SpawnSpec {
    Bin(String),
    Path(String),
}

/// One spawn request: (pid, parent_pid, spec, args, uid, username). Filled from context in Lua callbacks.
pub type SpawnQueueItem = (u64, u64, SpawnSpec, Vec<String>, i32, String);

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

    /// Next PID to allocate (synced from OS at start of VM tick). Lua spawn callbacks use this and increment.
    pub next_pid: u64,

    /// Queue of spawn requests (pid, parent_pid, spec, args, uid, username). Drained after tick.
    pub spawn_queue: Vec<SpawnQueueItem>,

    /// Snapshot of process status by PID: "running" | "finished". Built at start of VM tick.
    pub process_status_map: HashMap<u64, String>,

    /// (pid, line) to inject into process stdin. Applied after tick.
    pub stdin_inject_queue: Vec<(u64, String)>,

    /// Snapshot of process stdout by PID. Built at start of VM tick.
    pub process_stdout: HashMap<u64, String>,

    /// Stdout of processes that finished in the previous tick (so os.read_stdout(pid) works once after exit).
    pub last_stdout_of_finished: HashMap<u64, String>,
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
            next_pid: 1,
            spawn_queue: Vec::new(),
            process_status_map: HashMap::new(),
            stdin_inject_queue: Vec::new(),
            process_stdout: HashMap::new(),
            last_stdout_of_finished: HashMap::new(),
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
        self.process_status_map.clear();
        self.stdin_inject_queue.clear();
        self.process_stdout.clear();
    }

    /// Call after building process_stdout from current processes so os.read_stdout(pid) works for just-finished PIDs.
    pub fn merge_last_stdout_of_finished(&mut self) {
        for (pid, s) in self.last_stdout_of_finished.drain() {
            self.process_stdout.insert(pid, s);
        }
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
