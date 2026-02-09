#![allow(unused_variables)]
#![allow(dead_code)]
use super::os::OS;
use mlua::Lua;
use uuid::Uuid;

pub struct VirtualMachine<'a> {
    pub id: Uuid,
    pub os: OS<'a>,
    lua: &'a Lua,
}

impl<'a> VirtualMachine<'a> {
    pub fn new(lua: &'a Lua) -> Self {
        Self {
            id: Uuid::new_v4(),
            os: OS::new(&lua),
            lua,
        }
    }
}
