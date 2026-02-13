#![allow(unused_variables)]
#![allow(dead_code)]
use super::net::connection::ConnectionState;
use super::net::nic::NIC;
use super::os::OS;
use mlua::Lua;
use std::collections::HashMap;
use uuid::Uuid;

pub struct VirtualMachine<'a> {
    pub id: Uuid,
    pub os: OS<'a>,
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
    lua: &'a Lua,
}

/// Ticks per second from CPU cores (1 core=10, 2=20, 4=40, 8+=60 capped at game TPS).
pub fn ticks_per_second_from_cpu(cpu_cores: i16) -> u32 {
    let n = cpu_cores.max(1) as u32;
    (n * 10).min(60)
}

impl<'a> VirtualMachine<'a> {
    /// Create a VM with default cpu_cores=1 (e.g. stress test).
    pub fn new(lua: &'a Lua) -> Self {
        Self::with_id_and_cpu(lua, Uuid::new_v4(), 1)
    }

    /// Create a VM with a specific ID (for restoring from DB). Uses cpu_cores=1.
    pub fn with_id(lua: &'a Lua, id: Uuid) -> Self {
        Self::with_id_and_cpu(lua, id, 1)
    }

    /// Create a VM with a specific ID and CPU cores (for restoring from DB with processor info).
    pub fn with_id_and_cpu(lua: &'a Lua, id: Uuid, cpu_cores: i16) -> Self {
        let ticks_per_second = ticks_per_second_from_cpu(cpu_cores);
        Self {
            id,
            os: OS::new(lua),
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
}
