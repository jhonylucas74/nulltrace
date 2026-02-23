#![allow(dead_code)]

use super::context::VmContext;
use crate::db::fs_service::FsService;
use crate::file_search::{
    search_files, search_files_content, SearchContentOptions, SearchFilesOptions,
};
use mlua::{Lua, Result, Value};
use std::sync::Arc;

fn opts_to_search_files_options(opts: Option<mlua::Table>) -> Result<SearchFilesOptions> {
    let mut options = SearchFilesOptions::default();
    let Some(t) = opts else {
        return Ok(options);
    };
    if let Ok(v) = t.get::<String>("name") {
        options.name = Some(v);
    }
    if let Ok(v) = t.get::<String>("iname") {
        options.iname = Some(v);
    }
    if let Ok(v) = t.get::<String>("type") {
        options.type_filter = Some(v);
    }
    if let Ok(v) = t.get::<String>("size") {
        options.size_spec = Some(v);
    }
    if let Ok(v) = t.get::<String>("user") {
        options.user = Some(v);
    }
    if let Ok(v) = t.get::<mlua::Integer>("mtime") {
        options.mtime_days = Some(v as i64);
    }
    Ok(options)
}

fn opts_to_search_content_options(opts: Option<mlua::Table>) -> Result<SearchContentOptions> {
    let mut options = SearchContentOptions::default();
    let Some(t) = opts else {
        return Ok(options);
    };
    if let Ok(v) = t.get::<bool>("regex") {
        options.regex = v;
    }
    if let Ok(v) = t.get::<bool>("case_insensitive") {
        options.case_insensitive = v;
    }
    Ok(options)
}

fn value_to_path_list(v: &Value) -> Result<Vec<String>> {
    match v {
        Value::String(s) => Ok(vec![s.to_str()?.to_string()]),
        Value::Table(t) => {
            let mut paths = Vec::new();
            for pair in t.sequence_values::<String>() {
                paths.push(pair?);
            }
            Ok(paths)
        }
        _ => Err(mlua::Error::runtime(
            "search_files_content: paths must be string or table of strings",
        )),
    }
}

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

    // fs.ls_formatted(path) -> table of preformatted line strings (faster: one DB call, simple return).
    {
        let svc = fs_service.clone();
        fs.set(
            "ls_formatted",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let lines = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.ls_formatted(vm_id, &path).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let result = lua.create_table()?;
                for (i, line) in lines.iter().enumerate() {
                    result.set(i + 1, line.as_str())?;
                }
                Ok(Value::Table(result))
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

    // fs.read_lines(path) -> table of line strings | nil (avoids gmatch in Lua which can trigger yield across C boundary)
    {
        let svc = fs_service.clone();
        fs.set(
            "read_lines",
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
                        let s = String::from_utf8_lossy(&data);
                        let tbl = lua.create_table()?;
                        for (i, line) in s.lines().enumerate() {
                            tbl.set(i + 1, line)?;
                        }
                        Ok(mlua::Value::Table(tbl))
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

    // fs.search_files(path, opts) -> table of path strings. opts: name, iname, type, size, user, mtime (all optional).
    {
        let svc = fs_service.clone();
        fs.set(
            "search_files",
            lua.create_function(move |lua, (path, opts): (String, Option<mlua::Table>)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let options = opts_to_search_files_options(opts)?;
                let svc = svc.clone();
                let paths = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        search_files(&svc, vm_id, &path, &options).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let result = lua.create_table()?;
                for (i, p) in paths.iter().enumerate() {
                    result.set(i + 1, p.as_str())?;
                }
                Ok(Value::Table(result))
            })?,
        )?;
    }

    // fs.search_files_content(paths, pattern, opts) -> table of { path, line_num, line }. paths: string or table of strings. opts: regex?, case_insensitive?
    {
        let svc = fs_service.clone();
        fs.set(
            "search_files_content",
            lua.create_function(
                move |lua, (paths_arg, pattern, opts): (Value, String, Option<mlua::Table>)| {
                    let ctx = lua
                        .app_data_ref::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    let vm_id = ctx.vm_id;
                    drop(ctx);

                    let paths = value_to_path_list(&paths_arg)?;
                    let content_opts = opts_to_search_content_options(opts)?;
                    let svc = svc.clone();
                    let matches = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            search_files_content(&svc, vm_id, &paths, &pattern, &content_opts).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                    let result = lua.create_table()?;
                    for (i, m) in matches.iter().enumerate() {
                        let row = lua.create_table()?;
                        row.set("path", m.path.as_str())?;
                        row.set("line_num", m.line_num)?;
                        row.set("line", m.line.as_str())?;
                        result.set(i + 1, row)?;
                    }
                    Ok(Value::Table(result))
                },
            )?,
        )?;
    }

    lua.globals().set("fs", fs)?;
    Ok(())
}
