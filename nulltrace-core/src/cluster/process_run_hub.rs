//! Hub for "run process once": run a VM binary with args, stream stdout, then signal finished(exit_code).
//! Separate from TerminalStream and Process Spy; used by e.g. Code app for grep/find/sed.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Message sent from game loop to the gRPC RunProcess stream task.
pub enum RunProcessStreamMsg {
    Stdout(String),
    Finished(i32),
}

/// Pending run: (player_id, bin_name, args, response channel).
/// Game loop will spawn process and send response_tx a Receiver<RunProcessStreamMsg> so the gRPC task can stream responses.
pub type PendingRun = (
    Uuid,
    String,
    Vec<String>,
    oneshot::Sender<Result<mpsc::Receiver<RunProcessStreamMsg>, String>>,
);

/// Active run job: (sender to stream task, last_stdout_len for this process).
pub type ActiveRun = (mpsc::Sender<RunProcessStreamMsg>, usize);

pub struct ProcessRunHubInner {
    pub pending_runs: Vec<PendingRun>,
    /// Key: (vm_id, pid). Value: (tx to gRPC task, last_stdout_len).
    pub active_runs: HashMap<(Uuid, u64), ActiveRun>,
}

pub type ProcessRunHub = Mutex<ProcessRunHubInner>;

impl ProcessRunHubInner {
    pub fn new() -> Self {
        Self {
            pending_runs: Vec::new(),
            active_runs: HashMap::new(),
        }
    }
}

impl Default for ProcessRunHubInner {
    fn default() -> Self {
        Self::new()
    }
}

pub fn new_hub() -> std::sync::Arc<ProcessRunHub> {
    std::sync::Arc::new(Mutex::new(ProcessRunHubInner::new()))
}
