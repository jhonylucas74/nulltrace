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
    lua: &'a Lua,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(lua: &'a Lua) -> Self {
        Self {
            id: Uuid::new_v4(),
            os: OS::new(lua),
            nic: None,
            connections: HashMap::new(),
            next_connection_id: 1,
            lua,
        }
    }

    /// Create a VM with a specific ID (for restoring from DB).
    pub fn with_id(lua: &'a Lua, id: Uuid) -> Self {
        Self {
            id,
            os: OS::new(lua),
            nic: None,
            connections: HashMap::new(),
            next_connection_id: 1,
            lua,
        }
    }

    pub fn attach_nic(&mut self, nic: NIC) {
        self.nic = Some(nic);
    }
}
