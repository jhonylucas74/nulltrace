pub mod context;
pub mod fs_api;
pub mod net_api;
pub mod os_api;

use crate::db::fs_service::FsService;
use mlua::{Lua, Result};
use std::sync::Arc;

/// Register all Lua APIs (fs, net, os) on the shared Lua state.
pub fn register_all(lua: &Lua, fs_service: Arc<FsService>) -> Result<()> {
    fs_api::register(lua, fs_service)?;
    net_api::register(lua)?;
    os_api::register(lua)?;
    Ok(())
}
