//! Terminal session hub: bridges gRPC terminal connections to shell processes by PID.
//! Does not modify the shell; only interacts with existing process stdin/stdout.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Sent from the game loop to the gRPC handler when a terminal session is ready.
pub struct SessionReady {
    pub session_id: Uuid,
    /// VM that owns the shell process (for pending_kills when session ends).
    pub vm_id: Uuid,
    /// Shell process PID (for pending_kills when session ends).
    pub pid: u64,
    /// gRPC task receives shell stdout from this.
    pub stdout_rx: mpsc::Receiver<String>,
    /// gRPC task sends UI stdin to the game loop via this.
    pub stdin_tx: mpsc::Sender<String>,
    /// gRPC task receives error messages (e.g. memory limit, process terminated) from this.
    pub error_rx: mpsc::Receiver<String>,
}

/// Per-session state held by the hub; game loop uses this to bridge process I/O.
pub struct TerminalSession {
    pub vm_id: Uuid,
    pub pid: u64,
    /// Game loop sends new stdout chunks to the gRPC task.
    pub stdout_tx: mpsc::Sender<String>,
    /// Game loop drains this and pushes into the process's stdin.
    pub stdin_rx: mpsc::Receiver<String>,
    /// Game loop sends error messages (e.g. memory limit, process terminated) before closing.
    pub error_tx: mpsc::Sender<String>,
    /// Number of characters already sent on stdout_tx (process.stdout grows; we send suffix).
    pub last_stdout_len: usize,
}

/// Shared hub: pending open requests, active sessions, pending kills when a session ends, and pending interrupts (Ctrl+C).
pub struct TerminalHubInner {
    pub pending_opens: Vec<(Uuid, oneshot::Sender<Result<SessionReady, String>>)>,
    /// Code Run: (player_id, script path, response channel). Game loop spawns lua path and creates session like terminal.
    pub pending_code_runs: Vec<(Uuid, String, oneshot::Sender<Result<SessionReady, String>>)>,
    pub sessions: HashMap<Uuid, TerminalSession>,
    /// (vm_id, pid) to kill when the game loop runs (session closed); applied via kill_process_and_descendants.
    pub pending_kills: Vec<(Uuid, u64)>,
    /// (vm_id, shell_pid) for Ctrl+C; game loop kills only the shell's foreground child, not the shell.
    pub pending_interrupts: Vec<(Uuid, u64)>,
}

pub type TerminalHub = Mutex<TerminalHubInner>;

impl TerminalHubInner {
    pub fn new() -> Self {
        Self {
            pending_opens: Vec::new(),
            pending_code_runs: Vec::new(),
            sessions: HashMap::new(),
            pending_kills: Vec::new(),
            pending_interrupts: Vec::new(),
        }
    }
}

impl Default for TerminalHubInner {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new hub (wrap in Arc for sharing between game loop and gRPC).
pub fn new_hub() -> std::sync::Arc<TerminalHub> {
    std::sync::Arc::new(Mutex::new(TerminalHubInner::new()))
}
