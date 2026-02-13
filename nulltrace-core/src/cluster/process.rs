#![allow(unused_variables)]
#![allow(dead_code)]

use mlua::{Lua, Result, Thread, ThreadStatus};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Base memory (bytes) attributed to each process regardless of code size.
pub const BASE_MEMORY_PER_PROCESS_BYTES: u64 = 64 * 1024; // 64 KiB
/// Extra bytes per character of loaded Lua code (estimated memory).
pub const BYTES_PER_CODE_CHAR: u64 = 2;

/// Max lines in process stdin. Oldest dropped when exceeded.
pub const MAX_STDIN_LINES: usize = 256;

/// Max bytes in process stdout. Oldest truncated when exceeded.
pub const MAX_STDOUT_BYTES: usize = 64 * 1024; // 64 KiB

/// Push a line to stdin, dropping oldest if over MAX_STDIN_LINES.
pub fn push_stdin_line(guard: &mut VecDeque<String>, line: String) {
    while guard.len() >= MAX_STDIN_LINES {
        guard.pop_front();
    }
    guard.push_back(line);
}

/// Truncate stdout from the start if over MAX_STDOUT_BYTES.
pub fn truncate_stdout_if_needed(s: &mut String) {
    if s.len() > MAX_STDOUT_BYTES {
        let drop = s.len() - MAX_STDOUT_BYTES;
        s.drain(..drop);
    }
}

pub struct Process {
    pub id: u64,
    /// Parent process ID, if this process was spawned as a child.
    pub parent_id: Option<u64>,
    pub user_id: i32,
    pub username: String,
    pub args: Vec<String>,
    pub stdin: Arc<Mutex<VecDeque<String>>>,
    pub stdout: Arc<Mutex<String>>,
    /// When set, io.write/print in this process also append to this buffer (parent stdout).
    pub forward_stdout_to: Option<Arc<Mutex<String>>>,
    /// Estimated memory (base + code length * BYTES_PER_CODE_CHAR); computed once at spawn.
    pub estimated_memory_bytes: u64,
    /// Display name for monitor (e.g. "sh", "echo"). When set, snapshot uses this instead of args[0].
    pub display_name: Option<String>,
    thread: Thread,
    finished: bool,
    duration: Instant,
}

impl Process {
    /// Creates a process with the given id and optional parent_id (for child processes).
    pub fn new(
        lua: &Lua,
        id: u64,
        parent_id: Option<u64>,
        user_id: i32,
        username: &str,
        lua_code: &str,
        args: Vec<String>,
    ) -> Result<Self> {
        let thread = lua.create_thread(lua.load(lua_code).into_function()?)?;
        let estimated_memory_bytes = BASE_MEMORY_PER_PROCESS_BYTES
            .saturating_add(lua_code.len() as u64 * BYTES_PER_CODE_CHAR);

        Ok(Self {
            id,
            parent_id,
            user_id,
            username: username.to_string(),
            args,
            stdin: Arc::new(Mutex::new(VecDeque::new())),
            stdout: Arc::new(Mutex::new(String::new())),
            forward_stdout_to: None,
            estimated_memory_bytes,
            display_name: None,
            thread,
            finished: false,
            duration: Instant::now(),
        })
    }

    /// Returns Err(mlua::Error::MemoryError) when the VM's memory limit is exceeded.
    pub fn tick(&mut self) -> mlua::Result<()> {
        match self.thread.status() {
            ThreadStatus::Resumable => {
                if let Err(e) = self.thread.resume::<()>(()) {
                    self.finished = true;
                    return Err(e);
                }
            }
            ThreadStatus::Running => {
                // Process still running (yielded)
            }
            ThreadStatus::Error => {
                self.finished = true;
            }
            ThreadStatus::Finished => {
                self.finished = true;
            }
        }
        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Marks the process as finished. The caller must remove this process from the OS list
    /// immediately (e.g. via `retain`) so that the Luau `Thread` is dropped and the Lua thread
    /// is terminated and cannot keep running.
    pub fn kill(&mut self) {
        self.finished = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_new_with_parent_id() {
        let lua = Lua::new();
        let process =
            Process::new(&lua, 2, Some(1), 0, "root", "return", vec![]).expect("Process::new");
        assert_eq!(process.id, 2);
        assert_eq!(process.parent_id, Some(1));
    }

    #[test]
    fn test_process_new_without_parent_id() {
        let lua = Lua::new();
        let process =
            Process::new(&lua, 1, None, 0, "root", "return", vec![]).expect("Process::new");
        assert_eq!(process.id, 1);
        assert_eq!(process.parent_id, None);
    }

    #[test]
    fn test_estimated_memory_bytes_empty_code() {
        let lua = Lua::new();
        let process =
            Process::new(&lua, 1, None, 0, "root", "", vec![]).expect("Process::new");
        assert_eq!(
            process.estimated_memory_bytes,
            BASE_MEMORY_PER_PROCESS_BYTES,
            "empty code should use only base memory"
        );
    }

    #[test]
    fn test_estimated_memory_bytes_from_code_length() {
        let lua = Lua::new();
        // Valid Lua: "return" (6 chars)
        let code = "return";
        let process =
            Process::new(&lua, 1, None, 0, "root", code, vec![]).expect("Process::new");
        assert_eq!(
            process.estimated_memory_bytes,
            BASE_MEMORY_PER_PROCESS_BYTES + (code.len() as u64 * BYTES_PER_CODE_CHAR),
            "memory = base + code_len * BYTES_PER_CODE_CHAR"
        );
        // 100 characters of valid Lua (comment)
        let code100 = "-- " .to_string() + &"x".repeat(97);
        let process2 =
            Process::new(&lua, 2, None, 0, "user", &code100, vec![]).expect("Process::new");
        assert_eq!(
            process2.estimated_memory_bytes,
            BASE_MEMORY_PER_PROCESS_BYTES + (100 * BYTES_PER_CODE_CHAR),
        );
    }
}
