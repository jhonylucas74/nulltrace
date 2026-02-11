#![allow(unused_variables)]
#![allow(dead_code)]
use super::process::Process;
use mlua::{Lua, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
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

    pub fn is_finished(&mut self) -> bool {
        self.is_finished
    }

    pub fn spawn_process(&mut self, lua_code: &str, args: Vec<String>, user_id: i32, username: &str) {
        let id = self.next_process_id.fetch_add(1, Ordering::Relaxed);

        if let Some(process) = Process::new(&self.lua, id, user_id, username, lua_code, args).ok() {
            self.processes.push(process);
        }
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
