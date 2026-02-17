#![allow(dead_code)]

use super::context::VmContext;
use crate::db::fs_service::FsService;
use mlua::{Lua, Result};
use std::sync::Arc;

/// Register the `fs` table on the Lua state.
/// Functions use `lua.app_data()` to get the current VM context (vm_id + pool).
pub fn register(lua: &Lua, fs_service: Arc<FsService>) -> Result<()> {
    let fs = lua.create_table()?;

    // fs.stat(path) -> { type, size, owner, mtime } or nil if path does not exist. mtime = seconds since epoch.
    {
        let svc = fs_service.clone();
        fs.set(
            "stat",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let stat = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.stat_at(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                match stat {
                    Some(s) => {
                        let result = lua.create_table()?;
                        result.set("type", s.node_type.as_str())?;
                        result.set("size", s.size_bytes)?;
                        result.set("owner", s.owner.as_str())?;
                        result.set("mtime", s.updated_at.timestamp())?;
                        Ok(mlua::Value::Table(result))
                    }
                    None => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
    }

    // fs.ls(path) -> table of entries
    {
        let svc = fs_service.clone();
        fs.set(
            "ls",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let entries = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.ls(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let result = lua.create_table()?;
                for (i, entry) in entries.iter().enumerate() {
                    let t = lua.create_table()?;
                    t.set("name", entry.name.as_str())?;
                    t.set("type", entry.node_type.as_str())?;
                    t.set("size", entry.size_bytes)?;
                    t.set("permissions", entry.permissions.as_str())?;
                    t.set("owner", entry.owner.as_str())?;
                    result.set(i + 1, t)?;
                }
                Ok(result)
            })?,
        )?;
    }

    // fs.read(path) -> string | nil
    {
        let svc = fs_service.clone();
        fs.set(
            "read",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.read_file(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                match result {
                    Some((data, _mime)) => {
                        let s = String::from_utf8(data)
                            .unwrap_or_else(|e| {
                                // Binary files: return as lossy string
                                String::from_utf8_lossy(e.as_bytes()).into_owned()
                            });
                        Ok(mlua::Value::String(lua.create_string(&s)?))
                    }
                    None => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
    }

    // fs.read_bytes(path) -> buffer (raw bytes) | nil
    {
        let svc = fs_service.clone();
        fs.set(
            "read_bytes",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.read_file(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                match result {
                    Some((data, _mime)) => Ok(mlua::Value::String(lua.create_string(&data)?)),
                    None => Ok(mlua::Value::Nil),
                }
            })?,
        )?;
    }

    // fs.write(path, data, mime_type?)
    {
        let svc = fs_service.clone();
        fs.set(
            "write",
            lua.create_function(move |lua, (path, data, mime): (String, String, Option<String>)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                let owner = ctx.current_username.clone();
                drop(ctx);

                let svc = svc.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.write_file(vm_id, &path, data.as_bytes(), mime.as_deref(), &owner)
                            .await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(())
            })?,
        )?;
    }

    // fs.mkdir(path)
    {
        let svc = fs_service.clone();
        fs.set(
            "mkdir",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                let owner = ctx.current_username.clone();
                drop(ctx);

                let svc = svc.clone();
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.mkdir(vm_id, &path, &owner).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(())
            })?,
        )?;
    }

    // fs.rm(path) -> boolean
    {
        let svc = fs_service.clone();
        fs.set(
            "rm",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let deleted = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.rm(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(deleted)
            })?,
        )?;
    }

    lua.globals().set("fs", fs)?;
    Ok(())
}
