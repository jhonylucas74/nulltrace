#![allow(unused_variables)]
#![allow(dead_code)]
use super::process::Process;
use mlua::{Lua, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Lua heap limit per VM: 1 MB. 5,000 VMs × 1 MB = 5 GB total.
pub const LUA_MEMORY_LIMIT_BYTES: usize = 1024 * 1024;

pub struct OS {
    pub processes: Vec<Process>,
    next_process_id: AtomicU64,
    is_finished: bool,
    /// Round-robin index for tick_one_process.
    next_process_index: usize,
}

pub fn create_lua_state() -> Lua {
    let lua = Lua::new();
    let _ = lua.sandbox(true);

    let rust_print = lua
        .create_function(|_, args: mlua::Variadic<String>| {
            // println!("[Lua print]: {}", args.join(" "));
            Ok(())
        })
        .unwrap();

    lua.globals().set("print", rust_print).unwrap();

    let count = AtomicU64::new(0);
    const MAX_STACK_LEVEL: usize = 64;
    lua.set_interrupt(move |lua| {
        // Only yield when no C (Rust) frame is on the stack; avoids "yield across C-call boundary".
        for level in 0..=MAX_STACK_LEVEL {
            if let Some(what) = lua.inspect_stack(level, |debug| debug.source().what) {
                if what == "C" {
                    return Ok(VmState::Continue);
                }
            } else {
                break;
            }
        }
        if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    lua
}

/// Creates a minimal Lua state for a VM (stress test): sandbox, print, interrupt, memory limit. No APIs.
/// Use when fs/net/os are not needed (e.g. standalone stress binary).
pub fn create_vm_lua_state_minimal() -> Result<Lua, mlua::Error> {
    let lua = create_lua_state();
    lua.set_memory_limit(LUA_MEMORY_LIMIT_BYTES)?;
    Ok(lua)
}

impl OS {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_process_id: AtomicU64::new(1),
            is_finished: false,
            next_process_index: 0,
        }
    }

    /// Returns the next PID that will be allocated (for context sync at tick start).
    pub fn next_process_id(&self) -> u64 {
        self.next_process_id.load(Ordering::Relaxed)
    }

    pub fn is_finished(&mut self) -> bool {
        self.is_finished
    }

    /// Spawns a process with a pre-allocated id and optional parent. When forward_stdout_to is Some,
    /// io.write/print in the new process will also append to that buffer (parent stdout).
    /// display_name: when Some, used in process list snapshot instead of args[0]; args stay user-only.
    /// Returns Some(id) on success, None on failure.
    pub fn spawn_process_with_id(
        &mut self,
        lua: &Lua,
        id: u64,
        parent_id: Option<u64>,
        lua_code: &str,
        args: Vec<String>,
        user_id: i32,
        username: &str,
        forward_stdout_to: Option<Arc<Mutex<String>>>,
        display_name: Option<String>,
    ) -> Option<u64> {
        let mut process = Process::new(
            lua,
            id,
            parent_id,
            user_id,
            username,
            lua_code,
            args,
        )
        .ok()?;
        process.forward_stdout_to = forward_stdout_to;
        process.display_name = display_name;
        self.next_process_id
            .fetch_max(id + 1, Ordering::Relaxed);
        self.processes.push(process);
        Some(id)
    }

    /// Backward-compatible spawn: allocates next id and uses no parent.
    pub fn spawn_process(&mut self, lua: &Lua, lua_code: &str, args: Vec<String>, user_id: i32, username: &str) {
        let id = self.next_process_id.fetch_add(1, Ordering::Relaxed);
        let _ = self.spawn_process_with_id(lua, id, None, lua_code, args, user_id, username, None, None);
    }

    /// Kills the process with the given PID and all its descendants (children, grandchildren, etc.).
    /// Marks them as finished and immediately drops them (and their Luau threads) so that no Lua
    /// code can keep running.
    pub fn kill_process_and_descendants(&mut self, root_pid: u64) {
        let mut to_kill: std::collections::HashSet<u64> = std::collections::HashSet::new();
        to_kill.insert(root_pid);
        loop {
            let mut added = false;
            for p in &self.processes {
                if let Some(pid) = p.parent_id {
                    if to_kill.contains(&pid) && !to_kill.contains(&p.id) {
                        to_kill.insert(p.id);
                        added = true;
                    }
                }
            }
            if !added {
                break;
            }
        }
        for p in &mut self.processes {
            if to_kill.contains(&p.id) {
                p.kill();
            }
        }
        self.processes.retain(|p| !p.is_finished());
        self.is_finished = self.processes.iter().all(|proc| proc.is_finished());
    }

    /// Returns Err(mlua::Error::MemoryError) when any process exceeds the VM's memory limit.
    pub fn tick(&mut self) -> Result<(), mlua::Error> {
        for process in &mut self.processes {
            if !process.is_finished() {
                process.tick()?;
            }
        }

        self.processes.retain(|p| !p.is_finished());
        self.is_finished = self.processes.iter().all(|proc| proc.is_finished());
        Ok(())
    }

    /// Returns the index of the next process to run (round-robin), or None if none runnable.
    /// Caller must set VmContext and then call tick_process_at.
    pub fn get_next_tick_index(&mut self) -> Option<usize> {
        self.processes.retain(|p| !p.is_finished());
        self.is_finished = self.processes.iter().all(|proc| proc.is_finished());
        if self.processes.is_empty() {
            return None;
        }
        self.next_process_index %= self.processes.len();
        let idx = self.next_process_index;
        if self.processes[idx].is_finished() {
            return None;
        }
        Some(idx)
    }

    /// Tick the process at the given index. Call after get_next_tick_index and setting VmContext.
    /// Returns Err(mlua::Error::MemoryError) when the VM's memory limit is exceeded.
    pub fn tick_process_at(&mut self, idx: usize) -> Result<(), mlua::Error> {
        if idx < self.processes.len() && !self.processes[idx].is_finished() {
            self.processes[idx].tick()?;
        }
        self.next_process_index = (idx + 1) % self.processes.len().max(1);
        Ok(())
    }

    /// Run exactly one process (round-robin). Returns true if a process was ticked.
    /// Used when caller does not need to set per-process context (e.g. tests).
    /// Returns Err when memory limit exceeded.
    pub fn tick_one_process(&mut self) -> Result<bool, mlua::Error> {
        if let Some(idx) = self.get_next_tick_index() {
            self.tick_process_at(idx)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn run(&mut self) -> u128 {
        let start = Instant::now();

        let mut i = 1;
        while self.processes.iter().any(|proc| !proc.is_finished()) {
            let _ = self.tick();
            i += 1;
            if i > 1000 {
                break;
            }
        }

        start.elapsed().as_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_process_id_initial() {
        let lua = create_lua_state();
        let os = OS::new();
        assert_eq!(os.next_process_id(), 1);
    }

    #[test]
    fn test_spawn_process_with_id_returns_pid_and_sets_parent() {
        let lua = create_lua_state();
        let mut os = OS::new();
        let result = os.spawn_process_with_id(&lua, 10, Some(1), "return", vec![], 0, "root", None, None);
        assert_eq!(result, Some(10));
        assert_eq!(os.processes.len(), 1);
        assert_eq!(os.processes[0].id, 10);
        assert_eq!(os.processes[0].parent_id, Some(1));
        assert!(os.next_process_id() >= 11);
    }

    #[test]
    fn test_spawn_process_with_id_updates_next_process_id() {
        let lua = create_lua_state();
        let mut os = OS::new();
        let _ = os.spawn_process_with_id(&lua, 5, None, "return", vec![], 0, "root", None, None);
        assert!(os.next_process_id() >= 6);
    }
}
