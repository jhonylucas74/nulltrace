#![allow(dead_code)]

use super::context::VmContext;
use mlua::{Lua, Result};

/// Register the `os` table on the Lua state.
pub fn register(lua: &Lua) -> Result<()> {
    let os = lua.create_table()?;

    // os.hostname() -> string
    os.set(
        "hostname",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.hostname.clone())
        })?,
    )?;

    // os.pid() -> number
    os.set(
        "pid",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.current_pid)
        })?,
    )?;

    // os.vm_id() -> string
    os.set(
        "vm_id",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.vm_id.to_string())
        })?,
    )?;

    lua.globals().set("os", os)?;
    Ok(())
}
