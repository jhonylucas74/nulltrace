#![allow(unused_variables)]
#![allow(dead_code)]
use super::net::connection::ConnectionState;
use super::net::nic::NIC;
use super::os::OS;
use mlua::Lua;
use std::collections::HashMap;
use uuid::Uuid;

pub struct VirtualMachine {
    pub id: Uuid,
    pub os: OS,
    pub nic: Option<NIC>,
    /// Connection state (net.connect) persisted per VM across ticks.
    pub connections: HashMap<u64, ConnectionState>,
    pub next_connection_id: u64,
    /// CPU cores for tick budget (from processor type).
    pub cpu_cores: i16,
    /// Remaining process ticks for the current second.
    pub remaining_ticks: u32,
    /// Max process ticks per second (derived from cpu_cores).
    pub ticks_per_second: u32,
    pub lua: Lua,
}

/// Ticks per second from CPU cores. Budget is reset every 0.5s, so effective TPS per VM = 2 * this value (40 × 2 = 80).
pub fn ticks_per_second_from_cpu(_cpu_cores: i16) -> u32 {
    40
}

impl VirtualMachine {
    /// Create a VM with default cpu_cores=1 (e.g. stress test).
    pub fn new(lua: Lua) -> Self {
        Self::with_id_and_cpu(lua, Uuid::new_v4(), 1)
    }

    /// Create a VM with a specific ID (for restoring from DB). Uses cpu_cores=1.
    pub fn with_id(lua: Lua, id: Uuid) -> Self {
        Self::with_id_and_cpu(lua, id, 1)
    }

    /// Create a VM with a specific ID and CPU cores (for restoring from DB with processor info).
    pub fn with_id_and_cpu(lua: Lua, id: Uuid, cpu_cores: i16) -> Self {
        let ticks_per_second = ticks_per_second_from_cpu(cpu_cores);
        Self {
            id,
            os: OS::new(),
            nic: None,
            connections: HashMap::new(),
            next_connection_id: 1,
            cpu_cores,
            remaining_ticks: ticks_per_second,
            ticks_per_second,
            lua,
        }
    }

    /// Returns true if the VM has at least one running process.
    pub fn has_running_processes(&self) -> bool {
        self.os.processes.iter().any(|p| !p.is_finished())
    }

    pub fn attach_nic(&mut self, nic: NIC) {
        self.nic = Some(nic);
    }

    /// Resets the Lua state after memory limit exceeded: clears processes, drops old Lua, replaces with new.
    /// The factory creates a fresh Lua (sandbox, APIs, VmContext, memory limit).
    pub fn reset_lua_state(&mut self, factory: impl FnOnce() -> Result<Lua, mlua::Error>) -> Result<(), mlua::Error> {
        self.os.processes.clear();
        let new_lua = factory()?;
        self.lua = new_lua;
        Ok(())
    }
}
