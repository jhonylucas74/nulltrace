//! Lua API for string/table utilities.
//! str.serialize_table(t) -> string (key=value lines for application/x-lua-table)
//! str.parse_table(s) -> table (parses key=value lines into a Lua table)

use mlua::{Lua, Result, Table, Value};

/// Register the `str` table.
pub fn register(lua: &Lua) -> Result<()> {
    let str_tbl = lua.create_table()?;

    // str.serialize_table(t) -> string. Converts a flat table to key=value lines (application/x-lua-table format).
    // Only string keys and string/number/boolean values are serialized. Skips nil and nested tables.
    str_tbl.set(
        "serialize_table",
        lua.create_function(|_lua, table: Table| {
            let mut lines = Vec::new();
            for pair in table.pairs::<Value, Value>() {
                let (k, v) = pair.map_err(|e| mlua::Error::runtime(e.to_string()))?;
                let key = value_to_string(&k)?;
                let val = value_to_string(&v)?;
                if let (Some(key), Some(val)) = (key, val) {
                    if !key.is_empty() {
                        lines.push(format!("{}={}", key, val));
                    }
                }
            }
            Ok(lines.join("\n"))
        })?,
    )?;

    // str.parse_table(s) -> table. Parses key=value lines (application/x-lua-table format) into a Lua table.
    // Each line "key=value" becomes t[key]=value. Skips lines without "=" or with empty key.
    str_tbl.set(
        "parse_table",
        lua.create_function(|lua, s: String| {
            let tbl = lua.create_table()?;
            if s.is_empty() {
                return Ok(tbl);
            }
            for line in s.lines() {
                let line = line.trim();
                if let Some(idx) = line.find('=') {
                    if idx > 0 {
                        let key = line[..idx].trim();
                        let val = line[idx + 1..].trim();
                        if !key.is_empty() {
                            tbl.set(key, val)?;
                        }
                    }
                }
            }
            Ok(tbl)
        })?,
    )?;

    lua.globals().set("str", str_tbl)?;
    Ok(())
}

fn value_to_string(v: &Value) -> Result<Option<String>> {
    match v {
        Value::String(s) => Ok(Some(s.to_str()?.to_string())),
        Value::Integer(n) => Ok(Some(n.to_string())),
        Value::Number(n) => Ok(Some(n.to_string())),
        Value::Boolean(b) => Ok(Some(b.to_string())),
        Value::Nil => Ok(None),
        _ => Ok(None),
    }
}
