#![allow(dead_code)]

use crate::net::connection::ConnectionState;
use crate::net::dns::DnsResolver;
use crate::net::ip::Ipv4Addr;
use crate::net::nic::NIC;
use crate::net::packet::Packet;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Ephemeral port range (IANA dynamic/private). Not in LISTEN; bound to a connection.
const EPHEMERAL_PORT_MIN: u16 = 49152;
const EPHEMERAL_PORT_MAX: u16 = 65535;

/// Maximum packets queued in VM's net_inbound (for listening sockets)
const MAX_VM_INBOUND_PACKETS: usize = 256;

/// Spawn target: from /bin by name or from a file path.
#[derive(Clone, Debug)]
pub enum SpawnSpec {
    Bin(String),
    Path(String),
}

/// One spawn request: (pid, parent_pid, spec, args, uid, username, forward_stdout). Filled from context in Lua callbacks.
pub type SpawnQueueItem = (u64, u64, SpawnSpec, Vec<String>, i32, String, bool);

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
    /// Port -> pid that owns it (from NIC at start of tick). Used by net.listen to reject same-port by another process.
    pub port_owners: HashMap<u16, u64>,
    /// (port, pid) to apply to NIC at end of tick.
    pub pending_listen: Vec<(u16, u64)>,

    /// Connection-based API: connection_id -> state.
    pub connections: HashMap<u64, ConnectionState>,
    pub next_connection_id: u64,
    /// Ports to register as ephemeral on NIC at end of tick.
    pub pending_ephemeral_register: Vec<u16>,
    /// Ports to unregister (conn:close or process exit).
    pub pending_ephemeral_unregister: Vec<u16>,

    /// stdin/stdout for the currently executing process (set before each tick)
    pub current_stdin: Option<Arc<Mutex<VecDeque<String>>>>,
    pub current_stdout: Option<Arc<Mutex<String>>>,
    /// When set, io.write/print also append to this buffer (native forward stdout to parent).
    pub current_stdout_forward: Option<Arc<Mutex<String>>>,

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

    /// Snapshot of process display name (or args[0]) by PID. Built at start of VM tick. Used by shell to detect e.g. "ssh".
    pub process_display_name: HashMap<u64, String>,

    /// Stdout of processes that finished in the previous tick (so os.read_stdout(pid) works once after exit).
    pub last_stdout_of_finished: HashMap<u64, String>,

    /// (vm_id, shell_pid) -> foreground child pid. Used by terminal Ctrl+C to kill only the foreground process.
    pub shell_foreground_pid: HashMap<(Uuid, u64), u64>,

    /// PIDs the shell (or any process) requested to kill this tick. Drained by game loop after tick; applied via kill_process_and_descendants.
    pub requested_kills: Vec<u64>,

    /// Current working directory per process (pid -> absolute path). Set when process is created; updated by os.chdir. Not cleared in set_vm.
    pub process_cwd: HashMap<u64, String>,

    /// (vm_id, shell_pid) that called os.prompt_ready() this tick. Drained by game loop to send prompt_ready to terminal clients.
    pub shell_prompt_ready_pending: HashSet<(Uuid, u64)>,

    /// Cluster DNS resolver for hostname resolution (ntml.org, haru.local, etc.). Set before each tick.
    pub dns_resolver: Option<Arc<std::sync::RwLock<DnsResolver>>>,
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
            port_owners: HashMap::new(),
            pending_listen: Vec::new(),
            connections: HashMap::new(),
            next_connection_id: 1,
            pending_ephemeral_register: Vec::new(),
            pending_ephemeral_unregister: Vec::new(),
            current_stdin: None,
            current_stdout: None,
            current_stdout_forward: None,
            process_args: Vec::new(),
            next_pid: 1,
            spawn_queue: Vec::new(),
            process_status_map: HashMap::new(),
            stdin_inject_queue: Vec::new(),
            process_stdout: HashMap::new(),
            process_display_name: HashMap::new(),
            last_stdout_of_finished: HashMap::new(),
            shell_foreground_pid: HashMap::new(),
            requested_kills: Vec::new(),
            process_cwd: HashMap::new(),
            shell_prompt_ready_pending: HashSet::new(),
            dns_resolver: None,
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
        self.port_owners.clear();
        self.pending_listen.clear();
        // connections and next_connection_id are swapped with VM each tick, not cleared here
        self.pending_ephemeral_register.clear();
        self.pending_ephemeral_unregister.clear();
        self.current_stdin = None;
        self.current_stdout = None;
        self.current_stdout_forward = None;
        self.process_args.clear();
        self.spawn_queue.clear();
        self.process_status_map.clear();
        self.stdin_inject_queue.clear();
        self.process_stdout.clear();
        self.process_display_name.clear();
        self.requested_kills.clear();
    }

    /// Set port ownership snapshot from the NIC (call after set_vm when VM has a NIC).
    pub fn set_port_owners(&mut self, owners: HashMap<u16, u64>) {
        self.port_owners = owners;
    }

    /// Allocate an ephemeral port not in port_owners and not used by any connection. Returns None if exhausted.
    pub fn alloc_ephemeral_port(&self) -> Option<u16> {
        let in_use: std::collections::HashSet<u16> = self
            .port_owners
            .keys()
            .chain(self.connections.values().map(|c| &c.local_port))
            .copied()
            .collect();
        (EPHEMERAL_PORT_MIN..=EPHEMERAL_PORT_MAX).find(|p| !in_use.contains(p))
    }

    /// Drain NIC ephemeral queues into each connection's inbound. Call at VM tick start (after loading net_inbound).
    pub fn sync_connection_inbounds_from_nic(&mut self, nic: &mut NIC) {
        for conn in self.connections.values_mut() {
            let mut temp = VecDeque::new();
            nic.drain_ephemeral_into(conn.local_port, &mut temp);

            // Apply limit when draining from NIC to connection
            for pkt in temp {
                conn.push_inbound(pkt);
            }
        }
    }

    /// Close all connections owned by this pid; push their local ports to pending_ephemeral_unregister.
    pub fn close_connections_for_pid(&mut self, pid: u64) {
        let ports: Vec<u16> = self
            .connections
            .iter()
            .filter(|(_, c)| c.pid == pid)
            .map(|(_, c)| c.local_port)
            .collect();
        for port in ports {
            self.pending_ephemeral_unregister.push(port);
        }
        self.connections.retain(|_, c| c.pid != pid);
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
        forward_stdout_to: Option<Arc<Mutex<String>>>,
    ) {
        self.current_stdin = Some(stdin);
        self.current_stdout = Some(stdout);
        self.current_stdout_forward = forward_stdout_to;
        self.process_args = args;
    }

    /// Push packet to net_inbound with limit enforcement
    pub fn push_inbound(&mut self, packet: Packet) {
        if self.net_inbound.len() >= MAX_VM_INBOUND_PACKETS {
            self.net_inbound.pop_front();
        }
        self.net_inbound.push_back(packet);
    }
}
