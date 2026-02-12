#![allow(unused_variables)]
#![allow(dead_code)]

use mlua::{Lua, Result, Thread, ThreadStatus};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

        Ok(Self {
            id,
            parent_id,
            user_id,
            username: username.to_string(),
            args,
            stdin: Arc::new(Mutex::new(VecDeque::new())),
            stdout: Arc::new(Mutex::new(String::new())),
            forward_stdout_to: None,
            thread,
            finished: false,
            duration: Instant::now(),
        })
    }

    pub fn tick(&mut self) {
        match self.thread.status() {
            ThreadStatus::Resumable => {
                let _ = self.thread.resume::<()>(());
            }
            ThreadStatus::Running => {
                // println!("Process still running!");
            }
            ThreadStatus::Error => {
                self.finished = true;
            }
            ThreadStatus::Finished => {
                self.finished = true;
                // println!("Process finished total time: {}", self.duration.elapsed().as_millis())
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
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
}
