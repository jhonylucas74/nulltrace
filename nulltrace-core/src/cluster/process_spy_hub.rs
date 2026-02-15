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

/// Shared hub: connection_id -> connection state.
pub struct ProcessSpyHubInner {
    pub connections: HashMap<Uuid, ProcessSpyConnection>,
}

pub type ProcessSpyHub = Mutex<ProcessSpyHubInner>;

impl ProcessSpyHubInner {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
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
