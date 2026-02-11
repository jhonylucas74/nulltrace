#![allow(dead_code)]

use super::context::VmContext;
use crate::db::user_service::UserService;
use mlua::{Lua, Result, Value};
use std::sync::Arc;

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

    // os.exec(name, args?) -> queues program to spawn from /bin/<name>
    os.set(
        "exec",
        lua.create_function(|lua, (name, args): (String, Option<mlua::Table>)| {
            let mut ctx = lua
                .app_data_mut::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let current_uid = ctx.current_uid;
            let current_username = ctx.current_username.clone();
            let argv: Vec<String> = match args {
                Some(t) => {
                    let mut v = Vec::new();
                    let mut i = 1;
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
                    v
                }
                None => Vec::new(),
            };
            ctx.spawn_queue.push((name, argv, current_uid, current_username));
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
