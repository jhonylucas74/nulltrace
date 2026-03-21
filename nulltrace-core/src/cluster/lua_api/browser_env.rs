//! Restricted Lua environment for browser processes.
//!
//! `create_browser_env` builds a sandboxed environment table that exposes only:
//!  - Safe Lua builtins (math, string, table, type, tostring, tonumber, pairs, ipairs,
//!    pcall, xpcall, error, assert, select, unpack/table.unpack, next, rawget, rawset,
//!    rawequal, rawlen, setmetatable, getmetatable)
//!  - `io.read` / `io.write` (routed through VmContext stdin/stdout)
//!  - `print` → `{"type":"print","message":"..."}` JSON line on stdout
//!  - `ui.*` → JSON patch lines on stdout
//!  - `browser.request_card(origin, callback)` → JSON line on stdout
//!  - `storage.set(key, value)` / `storage.get(key)` / `storage.remove(key)` /
//!    `storage.keys()` / `storage.clear()` — per-session in-memory key-value store
//!  - `json_encode(t)` / `json_decode(s)` via serde_json
//!  - `str.parse_table` / `str.serialize_table`
//!
//! Dangerous globals (fs, net, os, mail, fkebank, crypto, load, dofile, require, …)
//! are intentionally excluded.
//!
//! The while-loop event dispatcher is appended to user code by the game loop before
//! creating the process; the constant `BROWSER_WHILE_LOOP_INJECTION` holds that snippet.

use super::context::VmContext;
use crate::process::truncate_stdout_if_needed;
use mlua::{Lua, Result, Table, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Lua while-loop injected after user code to drive the event dispatch loop.
/// Also defines `http.*` via a proxy mechanism (frontend executes real HTTP calls).
pub const BROWSER_WHILE_LOOP_INJECTION: &str = r#"
-- Injected by NullTrace browser runtime
local _form_values  = {}
local _pending_evts = {}
local _http_counter = 0

-- HTTP client: writes http_request to stdout, polls stdin for http_response.
-- Events that arrive while waiting are buffered in _pending_evts.
local function _http_req(method, url, body, headers)
  _http_counter = _http_counter + 1
  local _id = tostring(_http_counter)
  io.write(json_encode({type="http_request",id=_id,method=method,url=url,body=body,headers=headers}) .. "\n")
  while true do
    local _l = io.read()
    if _l ~= nil and _l ~= "" then
      local _ok, _m = pcall(json_decode, _l)
      if _ok and type(_m) == "table" then
        if _m.type == "http_response" and _m.id == _id then
          return {status=_m.status, body=_m.body, headers=_m.headers or {}}
        else
          _pending_evts[#_pending_evts + 1] = _l
        end
      end
    end
  end
end
http = {
  get    = function(url, h)        return _http_req("GET",    url, nil,  h) end,
  post   = function(url, body, h)  return _http_req("POST",   url, body, h) end,
  put    = function(url, body, h)  return _http_req("PUT",    url, body, h) end,
  patch  = function(url, body, h)  return _http_req("PATCH",  url, body, h) end,
  delete = function(url, h)        return _http_req("DELETE", url, nil,  h) end,
}

if ui then
  function ui.get_value(name) return _form_values[name] end
end
while true do
  local _line
  if #_pending_evts > 0 then
    _line = table.remove(_pending_evts, 1)
  else
    _line = io.read()
  end
  if _line ~= nil and _line ~= "" then
    local _ok, _msg = pcall(json_decode, _line)
    if _ok and type(_msg) == "table" and _msg.type == "event" then
      _form_values = _msg.form_values or {}
      local _fn = _ENV[_msg.action]
      if type(_fn) == "function" then
        pcall(_fn, {
          eventData   = _msg.event_data or {},
          formValues  = _msg.form_values or {},
          targetId    = (_msg.event_data or {}).id,
        })
      end
    end
  end
end
"#;

/// Write a JSON line to the current process stdout.
fn write_json_line(lua: &Lua, json: &str) -> Result<()> {
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
    let line = format!("{}\n", json);
    out.push_str(&line);
    truncate_stdout_if_needed(&mut out);
    if let Some(ref f) = forward {
        let mut guard = f.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
        guard.push_str(&line);
        truncate_stdout_if_needed(&mut guard);
    }
    Ok(())
}

/// Convert a Lua Value to serde_json::Value (best-effort; tables become objects or arrays).
fn lua_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Nil => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(n) => serde_json::Value::Number((*n).into()),
        Value::Number(n) => serde_json::json!(*n),
        Value::String(s) => {
            let text = s.to_str().map(|x| x.to_string()).unwrap_or_default();
            serde_json::Value::String(text)
        }
        Value::Table(t) => {
            // Check if it looks like an array (sequential integer keys starting at 1)
            let len = t.raw_len();
            if len > 0 {
                let arr: Vec<serde_json::Value> = (1..=len)
                    .map(|i| {
                        t.raw_get::<Value>(i)
                            .map(|v| lua_to_json(&v))
                            .unwrap_or(serde_json::Value::Null)
                    })
                    .collect();
                serde_json::Value::Array(arr)
            } else {
                let mut obj = serde_json::Map::new();
                if let Ok(pairs) = t
                    .clone()
                    .pairs::<Value, Value>()
                    .collect::<Result<Vec<_>>>()
                {
                    for (k, v) in pairs {
                        let key = match &k {
                            Value::String(s) => {
                                s.to_str().map(|x| x.to_string()).unwrap_or_default()
                            }
                            Value::Integer(n) => n.to_string(),
                            _ => continue,
                        };
                        obj.insert(key, lua_to_json(&v));
                    }
                }
                serde_json::Value::Object(obj)
            }
        }
        _ => serde_json::Value::Null,
    }
}

/// Convert serde_json::Value to a Lua Value.
fn json_to_lua(lua: &Lua, v: &serde_json::Value) -> Result<Value> {
    match v {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else {
                Ok(Value::Number(n.as_f64().unwrap_or(0.0)))
            }
        }
        serde_json::Value::String(s) => {
            Ok(Value::String(lua.create_string(s.as_bytes())?))
        }
        serde_json::Value::Array(arr) => {
            let t = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                t.raw_set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(Value::Table(t))
        }
        serde_json::Value::Object(obj) => {
            let t = lua.create_table()?;
            for (k, v) in obj {
                t.raw_set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(Value::Table(t))
        }
    }
}

/// Build and return the restricted environment table for browser processes.
///
/// The env is used with `lua.load(code).set_environment(env)` so the user code
/// only sees symbols present in this table.
///
/// `storage` is a per-session in-memory key-value store (set, get, remove, keys, clear).
pub fn create_browser_env(
    lua: &Lua,
    storage: Arc<Mutex<HashMap<String, String>>>,
) -> Result<Table> {
    let env = lua.create_table()?;
    let globals = lua.globals();

    // ── Safe Lua builtins ──────────────────────────────────────────────────
    for name in &[
        "math", "string", "table", "type", "tostring", "tonumber",
        "pairs", "ipairs", "next", "select",
        "pcall", "xpcall", "error", "assert",
        "rawget", "rawset", "rawequal", "rawlen",
        "setmetatable", "getmetatable",
        "unpack",
    ] {
        if let Ok(v) = globals.get::<Value>(*name) {
            env.set(*name, v)?;
        }
    }

    // ── str table (reuse registered global) ──────────────────────────────
    if let Ok(str_tbl) = globals.get::<Value>("str") {
        env.set("str", str_tbl)?;
    }

    // ── io table: read + write routed through VmContext ──────────────────
    let io_tbl = lua.create_table()?;

    io_tbl.set(
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
            let mut guard = stdin.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            let opt = guard.pop_front();
            drop(guard);
            Ok(opt
                .and_then(|s| lua.create_string(s.as_bytes()).ok().map(Value::String))
                .unwrap_or(Value::Nil))
        })?,
    )?;

    io_tbl.set(
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

    env.set("io", io_tbl)?;

    // ── print → {"type":"print","message":"..."} JSON line ────────────────
    env.set(
        "print",
        lua.create_function(|lua, args: mlua::Variadic<Value>| {
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
            let msg = parts.join("\t");
            let json = serde_json::json!({"type": "print", "message": msg}).to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    // ── json_encode / json_decode ─────────────────────────────────────────
    env.set(
        "json_encode",
        lua.create_function(|_lua, v: Value| {
            let jv = lua_to_json(&v);
            serde_json::to_string(&jv).map_err(|e| mlua::Error::runtime(e.to_string()))
        })?,
    )?;

    env.set(
        "json_decode",
        lua.create_function(|lua, s: String| {
            let jv: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| mlua::Error::runtime(e.to_string()))?;
            json_to_lua(lua, &jv)
        })?,
    )?;

    // ── ui table: patch operations written as JSON lines to stdout ────────
    let ui_tbl = lua.create_table()?;

    ui_tbl.set(
        "set_text",
        lua.create_function(|lua, (id, text): (String, String)| {
            let json = serde_json::json!({"type":"patch","op":"set_text","id":id,"text":text})
                .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "set_visible",
        lua.create_function(|lua, (id, visible): (String, bool)| {
            let json =
                serde_json::json!({"type":"patch","op":"set_visible","id":id,"visible":visible})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "set_class",
        lua.create_function(|lua, (id, class): (String, String)| {
            let json =
                serde_json::json!({"type":"patch","op":"set_class","id":id,"class":class})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "add_class",
        lua.create_function(|lua, (id, class): (String, String)| {
            let json =
                serde_json::json!({"type":"patch","op":"add_class","id":id,"class":class})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "remove_class",
        lua.create_function(|lua, (id, class): (String, String)| {
            let json =
                serde_json::json!({"type":"patch","op":"remove_class","id":id,"class":class})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "set_input_value",
        lua.create_function(|lua, (id, value): (String, String)| {
            let json =
                serde_json::json!({"type":"patch","op":"set_input_value","id":id,"value":value})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "set_value",
        lua.create_function(|lua, (id, value): (String, f64)| {
            let json =
                serde_json::json!({"type":"patch","op":"set_value","id":id,"value":value})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    ui_tbl.set(
        "set_disabled",
        lua.create_function(|lua, (id, disabled): (String, bool)| {
            let json =
                serde_json::json!({"type":"patch","op":"set_disabled","id":id,"disabled":disabled})
                    .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    // ui.get_value stub — overridden by the while-loop injection.
    ui_tbl.set(
        "get_value",
        lua.create_function(|_lua, _name: String| -> Result<Value> { Ok(Value::Nil) })?,
    )?;

    env.set("ui", ui_tbl)?;

    // ── storage table: per-session in-memory key-value store ───────────────
    let storage_clone = Arc::clone(&storage);
    let storage_tbl = lua.create_table()?;
    storage_tbl.set(
        "set",
        lua.create_function(move |_lua, (key, value): (String, String)| {
            let mut guard = storage_clone.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            guard.insert(key, value);
            Ok(())
        })?,
    )?;
    let storage_clone2 = Arc::clone(&storage);
    storage_tbl.set(
        "get",
        lua.create_function(move |lua, key: String| {
            let guard = storage_clone2.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            Ok(guard
                .get(&key)
                .and_then(|s| lua.create_string(s.as_bytes()).ok())
                .map(Value::String)
                .unwrap_or(Value::Nil))
        })?,
    )?;
    let storage_clone3 = Arc::clone(&storage);
    storage_tbl.set(
        "remove",
        lua.create_function(move |_lua, key: String| {
            let mut guard = storage_clone3.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            guard.remove(&key);
            Ok(())
        })?,
    )?;
    let storage_clone4 = Arc::clone(&storage);
    storage_tbl.set(
        "keys",
        lua.create_function(move |lua, ()| {
            let guard = storage_clone4.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            let keys: Vec<String> = guard.keys().cloned().collect();
            let tbl = lua.create_table()?;
            for (i, k) in keys.into_iter().enumerate() {
                tbl.raw_set(i + 1, lua.create_string(k.as_bytes())?)?;
            }
            Ok(tbl)
        })?,
    )?;
    let storage_clone5 = Arc::clone(&storage);
    storage_tbl.set(
        "clear",
        lua.create_function(move |_lua, ()| {
            let mut guard = storage_clone5.lock().map_err(|e| mlua::Error::runtime(e.to_string()))?;
            guard.clear();
            Ok(())
        })?,
    )?;
    env.set("storage", storage_tbl)?;

    // ── browser table ─────────────────────────────────────────────────────
    let browser_tbl = lua.create_table()?;

    browser_tbl.set(
        "request_card",
        lua.create_function(|lua, (origin, callback): (String, String)| {
            let request_id = uuid::Uuid::new_v4().to_string();
            let json = serde_json::json!({
                "type": "request_card",
                "request_id": request_id,
                "origin": origin,
                "callback": callback,
            })
            .to_string();
            write_json_line(lua, &json)
        })?,
    )?;

    env.set("browser", browser_tbl)?;

    // ── _ENV self-reference (needed by Lua code that references _ENV) ──────
    env.set("_ENV", env.clone())?;

    Ok(env)
}
