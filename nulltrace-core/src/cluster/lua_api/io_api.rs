#![allow(dead_code)]

use super::context::VmContext;
use crate::process::truncate_stdout_if_needed;
use mlua::{Lua, Result, Value};

/// Register the `io` table and override `print` to write to process stdout.
pub fn register(lua: &Lua) -> Result<()> {
    let io = lua.create_table()?;

    // io.read() -> pops one line from current_stdin, returns string or nil if empty
    io.set(
        "read",
        lua.create_function(|lua, ()| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let stdin = match &ctx.current_stdin {
                Some(s) => s.clone(),
                None => return Ok(Value::Nil),
            };
            drop(ctx);
            let mut line = stdin.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            let opt = line.pop_front();
            drop(line);
            Ok(opt
                .and_then(|s| lua.create_string(&s).ok().map(Value::String))
                .unwrap_or(Value::Nil))
        })?,
    )?;

    // io.write(s) -> appends to current_stdout; if current_stdout_forward set, also appends there (native forward).
    io.set(
        "write",
        lua.create_function(|lua, s: String| {
            let ctx = lua
                .app_data_ref::<VmContext>()
                .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
            let stdout = match &ctx.current_stdout {
                Some(s) => s.clone(),
                None => return Ok(()),
            };
            let forward = ctx.current_stdout_forward.clone();
            drop(ctx);
            let mut out = stdout.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            out.push_str(&s);
            truncate_stdout_if_needed(&mut out);
            if let Some(ref f) = forward {
                let mut guard = f.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
                guard.push_str(&s);
                truncate_stdout_if_needed(&mut guard);
            }
            Ok(())
        })?,
    )?;

    lua.globals().set("io", io)?;

    // print(...) -> writes all args to current_stdout (tab-separated, newline at end); if current_stdout_forward set, also appends there.
    let print_fn = lua.create_function(|lua, args: mlua::Variadic<Value>| {
        let ctx = lua
            .app_data_ref::<VmContext>()
            .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
        let stdout = match &ctx.current_stdout {
            Some(s) => s.clone(),
            None => return Ok(()),
        };
        let forward = ctx.current_stdout_forward.clone();
        drop(ctx);
        let parts: Vec<String> = args
            .iter()
            .map(|v| match v {
                Value::String(s) => s.to_str().map(|x| x.to_string()).unwrap_or_default(),
                Value::Integer(n) => n.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Nil => "nil".to_string(),
                _ => format!("{:?}", v),
            })
            .collect();
        let line = parts.join("\t") + "\n";
        let mut out = stdout.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
        out.push_str(&line);
        truncate_stdout_if_needed(&mut out);
        if let Some(ref f) = forward {
            let mut guard = f.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            guard.push_str(&line);
            truncate_stdout_if_needed(&mut guard);
        }
        Ok(())
    })?;
    lua.globals().set("print", print_fn)?;

    Ok(())
}
