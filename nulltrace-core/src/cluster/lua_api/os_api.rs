#![allow(dead_code)]

use super::context::{SpawnSpec, VmContext};
use crate::db::fs_service::FsService;
use crate::path_util;
use crate::process_parser;
use crate::db::user_service::UserService;
use mlua::{Lua, Result, Value};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use uuid::Uuid;

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

/// Returns the longest common prefix of non-empty strings. If any is empty or they differ from start, returns the given prefix.
fn common_prefix(matches: &[String], prefix: &str) -> String {
    if matches.is_empty() {
        return prefix.to_string();
    }
    let mut common = matches[0].as_str();
    for m in &matches[1..] {
        let n = common.bytes().zip(m.bytes()).take_while(|(a, b)| a == b).count();
        common = &common[..n];
        if common.is_empty() {
            return prefix.to_string();
        }
    }
    common.to_string()
}

/// Autocomplete based on line and cwd: commands from /bin, paths from cwd. No ambiguity (single match or common prefix).
/// Returns the replacement full line, or None if no completion.
fn autocomplete_line(
    fs_service: &FsService,
    vm_id: Uuid,
    line: &str,
    cwd: &str,
) -> std::result::Result<Option<String>, sqlx::Error> {
    let rt = tokio::runtime::Handle::current();
    let line_trimmed = line.trim_end_matches('\t').trim_end();
    if line_trimmed.is_empty() {
        return Ok(None);
    }
    let tokens: Vec<&str> = line_trimmed.split_whitespace().collect();
    let last_token = match tokens.last() {
        Some(t) => *t,
        None => return Ok(None),
    };
    if last_token.is_empty() {
        return Ok(None);
    }

    let last_token_start = line_trimmed
        .len()
        .saturating_sub(last_token.len());
    let safe_start = last_token_start.min(line_trimmed.len());

    let replace_with = |completed: &str| -> String {
        format!("{}{}", &line_trimmed[..safe_start], completed)
    };

    // Path completion: last token contains /
    if last_token.contains('/') {
        let last_slash = last_token.rfind('/').unwrap();
        let dir_part = &last_token[..=last_slash]; // include slash e.g. "./" or "/tmp/"
        let prefix = &last_token[last_slash + 1..];
        let resolved_dir = path_util::resolve_relative(cwd, &last_token[..last_slash]);
        let entries = rt.block_on(fs_service.ls(vm_id, &resolved_dir))?;
        let matches: Vec<String> = entries
            .iter()
            .filter(|e| e.name.starts_with(prefix))
            .map(|e| e.name.clone())
            .collect();
        if matches.is_empty() {
            return Ok(None);
        }
        if matches.len() == 1 {
            return Ok(Some(replace_with(&format!("{}{}", dir_part, matches[0]))));
        }
        let common = common_prefix(&matches, prefix);
        if common != prefix {
            return Ok(Some(replace_with(&format!("{}{}", dir_part, common))));
        }
        return Ok(None);
    }

    // No slash: single token = command then file; multiple tokens = path in cwd
    if tokens.len() == 1 {
        // Try /bin (commands) first
        let bin_entries = rt.block_on(fs_service.ls(vm_id, "/bin"))?;
        let bin_matches: Vec<String> = bin_entries
            .iter()
            .filter(|e| e.name.starts_with(last_token))
            .map(|e| e.name.clone())
            .collect();
        if bin_matches.len() == 1 {
            return Ok(Some(replace_with(&bin_matches[0])));
        }
        if bin_matches.len() > 1 {
            let common = common_prefix(&bin_matches, last_token);
            if common != last_token {
                return Ok(Some(replace_with(&common)));
            }
        }
    }

    // Path in cwd (multiple tokens, or single token with no /bin match)
    let entries = rt.block_on(fs_service.ls(vm_id, cwd))?;
    let matches: Vec<String> = entries
        .iter()
        .filter(|e| e.name.starts_with(last_token))
        .map(|e| e.name.clone())
        .collect();
    if matches.is_empty() {
        return Ok(None);
    }
    if matches.len() == 1 {
        return Ok(Some(replace_with(&matches[0])));
    }
    let common = common_prefix(&matches, last_token);
    if common != last_token {
        return Ok(Some(replace_with(&common)));
    }
    Ok(None)
}

/// Register the `os` table on the Lua state.
pub fn register(lua: &Lua, user_service: Arc<UserService>, fs_service: Arc<FsService>) -> Result<()> {
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

    // os.get_home() -> string. Current user's home directory (e.g. "/root"). Fallback "/" if user not found.
    {
        let svc = user_service.clone();
        os.set(
            "get_home",
            lua.create_function(move |lua, ()| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                let username = ctx.current_username.clone();
                drop(ctx);

                let svc = svc.clone();
                let home = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_user(vm_id, &username).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(home.map(|u| u.home_dir).unwrap_or_else(|| "/".to_string()))
            })?,
        )?;
    }

    // os.path_resolve(base, rel) -> string. Resolves relative path against base (handles ".", "..", absolute rel).
    os.set(
        "path_resolve",
        lua.create_function(|_lua, (base, rel): (String, String)| {
            Ok(path_util::resolve_relative(&base, &rel))
        })?,
    )?;

    // os.get_work_dir() -> string. Current process's working directory (absolute path). Default "/" if not set.
    os.set(
        "get_work_dir",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let cwd = ctx
                .process_cwd
                .get(&ctx.current_pid)
                .cloned()
                .unwrap_or_else(|| "/".to_string());
            Ok(cwd)
        })?,
    )?;

    // os.chdir(path) -> (). Change current process's working directory. path is resolved against current cwd. Errors if not a directory.
    {
        let fs_svc = fs_service.clone();
        os.set(
            "chdir",
            lua.create_function(move |lua, path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                let current_pid = ctx.current_pid;
                let cwd = ctx
                    .process_cwd
                    .get(&current_pid)
                    .cloned()
                    .unwrap_or_else(|| "/".to_string());
                let resolved = path_util::resolve_relative(&cwd, &path);
                drop(ctx);

                let fs_svc = fs_svc.clone();
                let node_type = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        fs_svc.node_type_at(vm_id, &resolved).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                match node_type.as_deref() {
                    Some("directory") => {}
                    Some(_) => {
                        return Err(mlua::Error::runtime(format!(
                            "chdir: not a directory: {}",
                            path
                        )));
                    }
                    None => {
                        return Err(mlua::Error::runtime(format!(
                            "chdir: no such file or directory: {}",
                            path
                        )));
                    }
                }

                let mut ctx = lua
                    .app_data_mut::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                ctx.process_cwd.insert(current_pid, resolved);
                Ok(())
            })?,
        )?;
    }

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

    // os.set_foreground_pid(pid) -> set current process's foreground child (for Ctrl+C: kill only this child, not shell)
    os.set(
        "set_foreground_pid",
        lua.create_function(|lua, pid: u64| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let key = (ctx.vm_id, ctx.current_pid);
            ctx.shell_foreground_pid.insert(key, pid);
            Ok(())
        })?,
    )?;

    // os.clear_foreground_pid() -> clear current process's foreground child (e.g. when child exits)
    os.set(
        "clear_foreground_pid",
        lua.create_function(|lua, ()| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let key = (ctx.vm_id, ctx.current_pid);
            ctx.shell_foreground_pid.remove(&key);
            Ok(())
        })?,
    )?;

    // os.request_kill(pid) -> request kill of process and descendants; applied by game loop after tick
    os.set(
        "request_kill",
        lua.create_function(|lua, pid: u64| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            ctx.requested_kills.push(pid);
            Ok(())
        })?,
    )?;

    // os.get_process_display_name(pid) -> display name (or args[0]) or nil
    os.set(
        "get_process_display_name",
        lua.create_function(|lua, pid: u64| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let name = ctx.process_display_name.get(&pid).cloned();
            Ok(match name {
                Some(s) => Value::String(lua.create_string(&s)?),
                None => Value::Nil,
            })
        })?,
    )?;

    // Ctrl+C sequence: single byte 0x03 (ETX). Used by shell to decide kill_child vs forward (e.g. to ssh).
    const STDIN_CTRL_C: u8 = 0x03;
    // Tab sequence: line ends with 0x09 (TAB). Used for autocomplete: forward (ssh) or discard (non-ssh).
    const STDIN_TAB: u8 = 0x09;

    // os.handle_special_stdin(line, child_pid) -> "kill_child" | "forward" | "discard" | "pass"
    os.set(
        "handle_special_stdin",
        lua.create_function(|lua, (line, child_pid): (String, Option<u64>)| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let pid = match child_pid {
                Some(p) if p != 0 => p,
                _ => return Ok(Value::String(lua.create_string("pass")?)),
            };
            let bytes = line.as_bytes();
            let is_ctrl_c = bytes == [STDIN_CTRL_C];
            let is_tab = bytes.contains(&STDIN_TAB);

            if is_ctrl_c {
                if !ctx.process_status_map.contains_key(&pid) {
                    return Ok(Value::String(lua.create_string("pass")?));
                }
                let name = ctx.process_display_name.get(&pid).map(|s| s.as_str()).unwrap_or("");
                if name == "ssh" {
                    return Ok(Value::String(lua.create_string("forward")?));
                }
                return Ok(Value::String(lua.create_string("kill_child")?));
            }
            if is_tab {
                if !ctx.process_status_map.contains_key(&pid) {
                    return Ok(Value::String(lua.create_string("pass")?));
                }
                let name = ctx.process_display_name.get(&pid).map(|s| s.as_str()).unwrap_or("");
                if name == "ssh" {
                    return Ok(Value::String(lua.create_string("forward")?));
                }
                return Ok(Value::String(lua.create_string("discard")?));
            }
            Ok(Value::String(lua.create_string("pass")?))
        })?,
    )?;

    // os.autocomplete(line, cwd) -> replacement line or nil. Resolves in Rust from /bin and cwd; no ambiguity (single match or common prefix).
    {
        let fs_svc = fs_service.clone();
        os.set(
            "autocomplete",
            lua.create_function(move |lua, (line, cwd): (String, String)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);
                // On any error or panic (e.g. fs.ls, no runtime, invalid path), return nil so the shell keeps running.
                let result = tokio::task::block_in_place(|| {
                    panic::catch_unwind(AssertUnwindSafe(|| {
                        autocomplete_line(&fs_svc, vm_id, &line, &cwd).ok().flatten()
                    }))
                    .unwrap_or(None)
                });
                Ok(match result {
                    Some(s) => Value::String(lua.create_string(&s)?),
                    None => Value::Nil,
                })
            })?,
        )?;
    }

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
