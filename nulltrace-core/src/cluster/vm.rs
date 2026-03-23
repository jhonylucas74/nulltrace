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
    /// Nominal RAM in MB (for Lua heap limit via nominal→real mapping when resetting).
    pub memory_mb: i32,
    /// Last measured CPU utilization (0–100) for player VMs; updated by the game loop. NPC/stress VMs stay 0.
    pub cpu_utilization_percent: u8,
    pub lua: Lua,
}

/// Budget units per core per 500ms window (same units as `ticks_per_second` / `remaining_ticks`).
/// Total VM budget = `TICKS_PER_CORE_PER_BUDGET * cpu_cores`. Each process may consume at most
/// `TICKS_PER_CORE_PER_BUDGET` per window (one "logical core" worth), leaving headroom when
/// `cpu_cores > 1` so System Monitor can show realistic aggregate and per-process CPU %.
pub const TICKS_PER_CORE_PER_BUDGET: u32 = 50;

/// Total tick budget for the VM (sum of per-core budgets). Scales linearly with core count.
pub fn ticks_per_second_from_cpu(cpu_cores: i16) -> u32 {
    let cores = cpu_cores.max(1) as u32;
    TICKS_PER_CORE_PER_BUDGET.saturating_mul(cores)
}

/// Per-process CPU display (0–100): share of the **VM total** tick budget for the window.
/// Matches Resources tab semantics: e.g. 2 cores → budget 100; one process at one-core cap (50 ticks) → 50%.
#[inline]
pub fn process_cpu_utilization_percent(ticks_consumed_this_budget: u32, vm_ticks_per_second: u32) -> u32 {
    if vm_ticks_per_second == 0 {
        return 0;
    }
    ((ticks_consumed_this_budget as u64 * 100) / vm_ticks_per_second as u64).min(100) as u32
}

impl VirtualMachine {
    /// Create a VM with default cpu_cores=1 (e.g. stress test). Uses 512 MB nominal RAM.
    pub fn new(lua: Lua) -> Self {
        Self::with_id_cpu_memory(lua, Uuid::new_v4(), 1, 512)
    }

    /// Create a VM with a specific ID (for restoring from DB). Uses cpu_cores=1, memory_mb=512.
    pub fn with_id(lua: Lua, id: Uuid) -> Self {
        Self::with_id_and_cpu(lua, id, 1)
    }

    /// Create a VM with a specific ID and CPU cores (for restoring from DB with processor info).
    /// Uses memory_mb=512. For production restore, use with_id_cpu_memory.
    pub fn with_id_and_cpu(lua: Lua, id: Uuid, cpu_cores: i16) -> Self {
        Self::with_id_cpu_memory(lua, id, cpu_cores, 512)
    }

    /// Create a VM with ID, CPU cores, and nominal RAM (for restoring from DB).
    pub fn with_id_cpu_memory(lua: Lua, id: Uuid, cpu_cores: i16, memory_mb: i32) -> Self {
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
            memory_mb,
            cpu_utilization_percent: 0,
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

    /// Spawn a process on this VM (avoids split borrows when calling from tests).
    pub fn spawn_process(&mut self, lua_code: &str, args: Vec<String>, user_id: i32, username: &str) {
        self.os.spawn_process(&self.lua, lua_code, args, user_id, username);
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
