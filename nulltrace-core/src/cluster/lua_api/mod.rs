pub mod context;
pub mod fs_api;
pub mod http_api;
pub mod httpd_api;
pub mod io_api;
pub mod net_api;
pub mod os_api;

use crate::db::fs_service::FsService;
use crate::db::user_service::UserService;
use mlua::{Lua, Result};
use std::sync::Arc;

/// Register all Lua APIs (fs, net, os, io) and safe globals (load) on the shared Lua state.
pub fn register_all(lua: &Lua, fs_service: Arc<FsService>, user_service: Arc<UserService>) -> Result<()> {
    fs_api::register(lua, fs_service.clone())?;
    net_api::register(lua)?;
    http_api::register(lua)?;
    httpd_api::register(lua, fs_service.clone())?;
    os_api::register(lua, user_service, fs_service)?;
    io_api::register(lua)?;
    // Expose load(source, chunkname?, mode?) so /bin/lua can run user scripts. Sandbox may not expose it.
    let load_fn = lua.create_function(|lua, (source, chunkname, _mode): (String, Option<String>, Option<String>)| {
        let chunkname = chunkname.unwrap_or_else(|| "=(load)".to_string());
        let fn_result = lua.load(&source).set_name(&chunkname).into_function();
        match fn_result {
            Ok(f) => Ok(mlua::Value::Function(f)),
            Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
        }
    })?;
    lua.globals().set("load", load_fn)?;
    Ok(())
}
