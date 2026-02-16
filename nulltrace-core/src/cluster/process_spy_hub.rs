//! Process Spy hub: bridges gRPC ProcessSpyStream connections to VM process stdin/stdout.
//! One connection per client; each connection can subscribe to multiple PIDs and receive
//! process list snapshots, stdout, and stdin chunks in real time.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::vm_manager::ProcessSnapshot;

/// Messages sent from the game loop (or gRPC recv task) to the gRPC send task for one connection.
#[derive(Debug)]
pub enum ProcessSpyDownstreamMsg {
    ProcessList(Vec<ProcessSnapshot>),
    Stdout(u64, String),
    StdinChunk(u64, String),
    ProcessGone(u64),
    /// Sent after spawning a Lua script via SpawnLuaScript; client subscribes to this pid.
    LuaScriptSpawned(u64),
    Error(String),
}

/// Per-PID subscription: game loop drains stdin_rx into process.stdin and drains process.stdout into downstream.
pub struct ProcessSpySubscription {
    /// gRPC recv task sends injected stdin via this; game loop drains stdin_rx and pushes to process.stdin.
    pub stdin_tx: mpsc::Sender<String>,
    /// Game loop drains this and pushes into process.stdin.
    pub stdin_rx: mpsc::Receiver<String>,
    /// Number of characters already sent (process.stdout suffix).
    pub last_stdout_len: usize,
}

/// One Process Spy connection (one client stream).
pub struct ProcessSpyConnection {
    pub player_id: Uuid,
    pub vm_id: Uuid,
    /// All messages to send to the client (process list, stdout, stdin chunks, process gone, error).
    pub downstream_tx: mpsc::Sender<ProcessSpyDownstreamMsg>,
    /// Subscribed PIDs for this connection.
    pub subscriptions: HashMap<u64, ProcessSpySubscription>,
    /// Whether we have sent at least one process list (so client gets list immediately from store).
    pub sent_initial_list: bool,
}

/// Pending "spawn lua script" request: (connection_id, vm_id, path).
pub type PendingLuaSpawn = (Uuid, Uuid, String);
/// Pending kill: (vm_id, pid).
pub type PendingKill = (Uuid, u64);

/// Max number of (vm_id, pid) stdout buffers to keep for late subscribers (e.g. Proc Spy opening a just-exited process).
const MAX_RECENTLY_FINISHED_STDOUT: usize = 20;

/// Shared hub: connection_id -> connection state, pending spawn/kill from client, and recently finished stdout for late subscribers.
pub struct ProcessSpyHubInner {
    pub connections: HashMap<Uuid, ProcessSpyConnection>,
    /// Spawn lua script requests; game loop drains and spawns, then sends LuaScriptSpawned(pid).
    pub pending_lua_spawns: Vec<PendingLuaSpawn>,
    /// Kill process requests; game loop drains and calls kill_process_and_descendants.
    pub pending_kills: Vec<PendingKill>,
    /// Stdout of recently finished processes so late subscribers (e.g. Proc Spy) can receive it. Capped to MAX_RECENTLY_FINISHED_STDOUT.
    pub recently_finished_stdout: HashMap<(Uuid, u64), String>,
}

pub type ProcessSpyHub = Mutex<ProcessSpyHubInner>;

impl ProcessSpyHubInner {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            pending_lua_spawns: Vec::new(),
            pending_kills: Vec::new(),
            recently_finished_stdout: HashMap::new(),
        }
    }

    /// Insert stdout for a finished (vm_id, pid). Evicts oldest entry if at capacity.
    pub fn insert_recently_finished_stdout(&mut self, vm_id: Uuid, pid: u64, stdout: String) {
        if self.recently_finished_stdout.len() >= MAX_RECENTLY_FINISHED_STDOUT {
            if let Some(key) = self.recently_finished_stdout.keys().next().copied() {
                self.recently_finished_stdout.remove(&key);
            }
        }
        self.recently_finished_stdout.insert((vm_id, pid), stdout);
    }
}

impl Default for ProcessSpyHubInner {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new hub (wrap in Arc for sharing between game loop and gRPC).
pub fn new_hub() -> std::sync::Arc<ProcessSpyHub> {
    std::sync::Arc::new(Mutex::new(ProcessSpyHubInner::new()))
}
