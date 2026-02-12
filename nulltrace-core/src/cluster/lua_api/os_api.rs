#![allow(dead_code)]

use super::context::{SpawnSpec, VmContext};
use crate::process_parser;
use crate::db::user_service::UserService;
use mlua::{Lua, Result, Value};
use std::sync::Arc;

/// Converts a Lua table (1-indexed sequence) to Vec<String> for argv.
fn table_to_argv(_lua: &Lua, t: Option<mlua::Table>) -> std::result::Result<Vec<String>, mlua::Error> {
    let mut v = Vec::new();
    let t = match t {
        Some(t) => t,
        None => return Ok(v),
    };
    let mut i = 1u32;
    loop {
        let val: Value = t.get(i)?;
        if let Value::Nil = val {
            break;
        }
        let s = match &val {
            Value::String(st) => st.to_str().map(|x| x.to_string()).unwrap_or_default(),
            Value::Integer(n) => n.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            _ => format!("{:?}", val),
        };
        v.push(s);
        i += 1;
    }
    Ok(v)
}

/// Register the `os` table on the Lua state.
pub fn register(lua: &Lua, user_service: Arc<UserService>) -> Result<()> {
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

    // os.whoami() -> string
    os.set(
        "whoami",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.current_username.clone())
        })?,
    )?;

    // os.uid() -> number
    os.set(
        "uid",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.current_uid)
        })?,
    )?;

    // os.is_root() -> boolean
    os.set(
        "is_root",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            Ok(ctx.current_uid == 0)
        })?,
    )?;

    // os.get_args() -> table of strings (Lua-indexed 1..n)
    os.set(
        "get_args",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let args = ctx.process_args.clone();
            drop(ctx);
            let result = lua.create_table()?;
            for (i, arg) in args.iter().enumerate() {
                result.set(i + 1, arg.as_str())?;
            }
            Ok(result)
        })?,
    )?;

    // os.spawn(name, args, options?) -> pid. options may have forward_stdout = true to forward child stdout to parent.
    os.set(
        "spawn",
        lua.create_function(
            |lua, (name, args, options): (String, Option<mlua::Table>, Option<mlua::Table>)| {
                let mut ctx = lua
                    .app_data_mut::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let argv = table_to_argv(&lua, args)?;
                let forward_stdout = options
                    .and_then(|t| t.get("forward_stdout").ok())
                    .and_then(|v: mlua::Value| v.as_boolean())
                    .unwrap_or(false);
                let pid = ctx.next_pid;
                ctx.next_pid = ctx.next_pid.saturating_add(1);
                let parent_pid = ctx.current_pid;
                let uid = ctx.current_uid;
                let username = ctx.current_username.clone();
                ctx.spawn_queue.push((
                    pid,
                    parent_pid,
                    SpawnSpec::Bin(name),
                    argv,
                    uid,
                    username,
                    forward_stdout,
                ));
                Ok(pid)
            },
        )?,
    )?;

    // os.spawn_path(path, args, options?) -> pid. options may have forward_stdout = true.
    os.set(
        "spawn_path",
        lua.create_function(
            |lua, (path, args, options): (String, Option<mlua::Table>, Option<mlua::Table>)| {
                let mut ctx = lua
                    .app_data_mut::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let argv = table_to_argv(&lua, args)?;
                let forward_stdout = options
                    .and_then(|t| t.get("forward_stdout").ok())
                    .and_then(|v: mlua::Value| v.as_boolean())
                    .unwrap_or(false);
                let pid = ctx.next_pid;
                ctx.next_pid = ctx.next_pid.saturating_add(1);
                let parent_pid = ctx.current_pid;
                let uid = ctx.current_uid;
                let username = ctx.current_username.clone();
                ctx.spawn_queue.push((
                    pid,
                    parent_pid,
                    SpawnSpec::Path(path),
                    argv,
                    uid,
                    username,
                    forward_stdout,
                ));
                Ok(pid)
            },
        )?,
    )?;

    // os.process_status(pid) -> "running" | "finished" | "scheduled" | "not_found"
    // "scheduled" = in spawn_queue this tick, not yet created (avoids not_found for just-spawned PIDs)
    os.set(
        "process_status",
        lua.create_function(|lua, pid: u64| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let status = if let Some(s) = ctx.process_status_map.get(&pid) {
                s.clone()
            } else if ctx.spawn_queue.iter().any(|(p, ..)| *p == pid) {
                "scheduled".to_string()
            } else {
                "not_found".to_string()
            };
            Ok(status)
        })?,
    )?;

    // os.write_stdin(pid, line) -> inject a line into process stdin
    os.set(
        "write_stdin",
        lua.create_function(|lua, (pid, line): (u64, String)| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            ctx.stdin_inject_queue.push((pid, line));
            Ok(())
        })?,
    )?;

    // os.read_stdout(pid) -> string or nil (includes stdout of just-finished processes from last_stdout_of_finished)
    os.set(
        "read_stdout",
        lua.create_function(|lua, pid: u64| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let out = ctx
                .process_stdout
                .get(&pid)
                .or_else(|| ctx.last_stdout_of_finished.get(&pid))
                .cloned();
            Ok(match out {
                Some(s) => Value::String(lua.create_string(&s)?),
                None => Value::Nil,
            })
        })?,
    )?;

    // os.parse_cmd(line) -> { program = string, args = table } or nil
    os.set(
        "parse_cmd",
        lua.create_function(|lua, line: String| {
            let (program, args) = process_parser::parse_cmd_line(&line);
            let t = lua.create_table()?;
            t.set("program", program.as_str())?;
            let args_t = lua.create_table()?;
            for (i, a) in args.iter().enumerate() {
                args_t.set(i + 1, a.as_str())?;
            }
            t.set("args", args_t)?;
            Ok(t)
        })?,
    )?;

    // os.exec(name, args?) -> queues program to spawn from /bin/<name> (fire-and-forget, no return)
    os.set(
        "exec",
        lua.create_function(|lua, (name, args): (String, Option<mlua::Table>)| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let argv = table_to_argv(&lua, args)?;
            let pid = ctx.next_pid;
            ctx.next_pid = ctx.next_pid.saturating_add(1);
            let parent_pid = ctx.current_pid;
            let uid = ctx.current_uid;
            let username = ctx.current_username.clone();
            ctx.spawn_queue.push((
                pid,
                parent_pid,
                SpawnSpec::Bin(name),
                argv,
                uid,
                username,
                false,
            ));
            Ok(())
        })?,
    )?;

    // os.users() -> table of { username, uid, home, shell, is_root }
    {
        let svc = user_service.clone();
        os.set(
            "users",
            lua.create_function(move |lua, ()| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let users = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.list_users(vm_id).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let result = lua.create_table()?;
                for (i, user) in users.iter().enumerate() {
                    let t = lua.create_table()?;
                    t.set("username", user.username.as_str())?;
                    t.set("uid", user.uid)?;
                    t.set("home", user.home_dir.as_str())?;
                    t.set("shell", user.shell.as_str())?;
                    t.set("is_root", user.is_root)?;
                    result.set(i + 1, t)?;
                }
                Ok(result)
            })?,
        )?;
    }

    // os.login(username, password) -> boolean
    {
        let svc = user_service.clone();
        os.set(
            "login",
            lua.create_function(move |lua, (username, password): (String, String)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let valid = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.verify_password(vm_id, &username, &password).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(valid)
            })?,
        )?;
    }

    // os.su(username, password) -> boolean
    // If credentials are valid, switches the current process's user identity.
    {
        let svc = user_service.clone();
        os.set(
            "su",
            lua.create_function(move |lua, (username, password): (String, String)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let svc = svc.clone();
                let user = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let valid = svc.verify_password(vm_id, &username, &password).await?;
                        if valid {
                            svc.get_user(vm_id, &username).await
                        } else {
                            Ok(None)
                        }
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                match user {
                    Some(u) => {
                        let mut ctx = lua
                            .app_data_mut::<VmContext>()
                            .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                        ctx.current_uid = u.uid;
                        ctx.current_username = u.username;
                        Ok(true)
                    }
                    None => Ok(false),
                }
            })?,
        )?;
    }

    lua.globals().set("os", os)?;
    Ok(())
}
