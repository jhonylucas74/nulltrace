//! Hub for "run process once": run a VM binary with args, stream stdout, then signal finished(exit_code).
//! Separate from TerminalStream and Process Spy; used by e.g. Code app for grep/find/sed.
//! Runs have a default timeout (e.g. 30s); when exceeded, the process is killed and for curl we send HTTP 504.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Default timeout for a run process (e.g. curl from browser). After this, process is killed and curl gets HTTP 504.
pub const RUN_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP response body sent when a curl run is killed due to timeout (so browser shows 504).
pub const CURL_TIMEOUT_HTTP_RESPONSE: &str =
    "HTTP/1.1 504 Gateway Timeout\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n# Request timeout (30s).\n";

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

/// Active run job: stream sender, last stdout length, start time, and bin name (for timeout response).
pub struct ActiveRun {
    pub stream_tx: mpsc::Sender<RunProcessStreamMsg>,
    pub last_stdout_len: usize,
    pub started_at: Instant,
    pub bin_name: String,
}

pub struct ProcessRunHubInner {
    pub pending_runs: Vec<PendingRun>,
    /// Key: (vm_id, pid). Value: active run state.
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
