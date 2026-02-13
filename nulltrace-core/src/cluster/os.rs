#![allow(unused_variables)]
#![allow(dead_code)]
use super::process::Process;
use mlua::{Lua, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct OS<'a> {
    pub processes: Vec<Process>,
    next_process_id: AtomicU64,
    is_finished: bool,
    lua: &'a Lua,
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
    lua.set_interrupt(move |_| {
        if count.fetch_add(1, Ordering::Relaxed) % 2 == 0 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    lua
}

impl<'a> OS<'a> {
    pub fn new(lua: &'a Lua) -> Self {
        Self {
            processes: Vec::new(),
            next_process_id: AtomicU64::new(1),
            is_finished: false,
            lua,
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
            &self.lua,
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
    pub fn spawn_process(&mut self, lua_code: &str, args: Vec<String>, user_id: i32, username: &str) {
        let id = self.next_process_id.fetch_add(1, Ordering::Relaxed);
        let _ = self.spawn_process_with_id(id, None, lua_code, args, user_id, username, None, None);
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

    pub fn tick(&mut self) {
        for process in &mut self.processes {
            if !process.is_finished() {
                process.tick();
            }
        }

        self.processes.retain(|p| !p.is_finished());
        self.is_finished = self.processes.iter().all(|proc| proc.is_finished());
    }

    pub fn run(&mut self) -> u128 {
        let start = Instant::now();

        let mut i = 1;
        while self.processes.iter().any(|proc| !proc.is_finished()) {
            self.tick();
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
        let os = OS::new(&lua);
        assert_eq!(os.next_process_id(), 1);
    }

    #[test]
    fn test_spawn_process_with_id_returns_pid_and_sets_parent() {
        let lua = create_lua_state();
        let mut os = OS::new(&lua);
        let result = os.spawn_process_with_id(10, Some(1), "return", vec![], 0, "root", None, None);
        assert_eq!(result, Some(10));
        assert_eq!(os.processes.len(), 1);
        assert_eq!(os.processes[0].id, 10);
        assert_eq!(os.processes[0].parent_id, Some(1));
        assert!(os.next_process_id() >= 11);
    }

    #[test]
    fn test_spawn_process_with_id_updates_next_process_id() {
        let lua = create_lua_state();
        let mut os = OS::new(&lua);
        let _ = os.spawn_process_with_id(5, None, "return", vec![], 0, "root", None, None);
        assert!(os.next_process_id() >= 6);
    }
}
