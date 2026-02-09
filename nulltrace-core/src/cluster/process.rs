#![allow(unused_variables)]
#![allow(dead_code)]

use mlua::{Lua, Result, Thread, ThreadStatus};
use std::time::Instant;

pub struct Process {
    pub id: u64,
    thread: Thread,
    finished: bool,
    duration: Instant,
}

impl Process {
    pub fn new(lua: &Lua, id: u64, lua_code: &str) -> Result<Self> {
        let thread = lua.create_thread(lua.load(lua_code).into_function()?)?;

        Ok(Self {
            id,
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

// impl Drop for Process {
//     fn drop(&mut self) {
//         // println!("Removing the process {}", self.id);
//     }
// }
