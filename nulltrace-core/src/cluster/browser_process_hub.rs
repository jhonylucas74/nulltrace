//! Browser process session hub: bridges gRPC BrowserProcessStream connections to isolated
//! Lua processes running in the player's VM. Mirrors terminal_hub.rs in structure.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Sent from the game loop to the gRPC handler when a browser session is ready.
pub struct BrowserSessionReady {
    pub session_id: Uuid,
    /// VM that owns the process (for pending_kills when session ends).
    pub vm_id: Uuid,
    /// Allocated PID of the browser process.
    pub pid: u64,
    /// gRPC task receives process stdout JSON lines from this.
    pub stdout_rx: mpsc::Receiver<String>,
    /// gRPC task sends input JSON lines (events / http_responses) to the game loop via this.
    pub stdin_tx: mpsc::Sender<String>,
    /// gRPC task receives error messages (e.g. memory limit, process terminated) from this.
    pub error_rx: mpsc::Receiver<String>,
}

/// Per-session state held by the hub; game loop uses this to bridge process I/O.
pub struct BrowserSession {
    pub vm_id: Uuid,
    pub pid: u64,
    /// Game loop sends new stdout JSON lines to the gRPC task.
    pub stdout_tx: mpsc::Sender<String>,
    /// Game loop drains this and injects into the process's stdin.
    pub stdin_rx: mpsc::Receiver<String>,
    /// Game loop sends error messages before closing.
    pub error_tx: mpsc::Sender<String>,
    /// Number of characters already forwarded on stdout_tx (process.stdout grows; we send suffix).
    pub last_stdout_len: usize,
}

/// Pending spawn: (player_id, lua_code, response_channel).
pub type PendingSpawn = (Uuid, String, oneshot::Sender<Result<BrowserSessionReady, String>>);

/// Shared hub accessed by both the game loop (in the main task) and gRPC handlers (in spawned tasks).
pub struct BrowserProcessHubInner {
    /// Requests to spawn a new browser process; game loop drains these each tick.
    pub pending_spawns: Vec<PendingSpawn>,
    /// Active sessions indexed by session_id.
    pub sessions: HashMap<Uuid, BrowserSession>,
    /// (vm_id, pid) to kill when the session ends; applied via kill_process_and_descendants.
    pub pending_kills: Vec<(Uuid, u64)>,
}

pub type BrowserProcessHub = Mutex<BrowserProcessHubInner>;

impl BrowserProcessHubInner {
    pub fn new() -> Self {
        Self {
            pending_spawns: Vec::new(),
            sessions: HashMap::new(),
            pending_kills: Vec::new(),
        }
    }
}

impl Default for BrowserProcessHubInner {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new hub wrapped in Arc for sharing between game loop and gRPC handlers.
pub fn new_hub() -> std::sync::Arc<BrowserProcessHub> {
    std::sync::Arc::new(Mutex::new(BrowserProcessHubInner::new()))
}
