//! Lua API for Fkebank (USD): transfer and history using token read from a file on the VM.

use super::context::VmContext;
use crate::db::fkebank_account_service::FkebankAccountService;
use crate::db::fs_service::FsService;
use mlua::{Lua, Result, Value};
use std::sync::Arc;

fn tx_to_lua_table(lua: &Lua, id: &uuid::Uuid, from_key: &str, to_key: &str, amount: i64, description: Option<&str>, created_at: i64) -> Result<Value> {
    let t = lua.create_table()?;
    t.set("id", id.to_string())?;
    t.set("from_key", from_key)?;
    t.set("to_key", to_key)?;
    t.set("amount", amount)?;
    t.set("description", description.unwrap_or(""))?;
    t.set("created_at_ms", created_at)?;
    Ok(Value::Table(t))
}

/// Register the `fkebank` table. Requires vm_id in VmContext (set when ticking the VM).
pub fn register(lua: &Lua, fs_service: Arc<FsService>, fkebank_service: Arc<FkebankAccountService>) -> Result<()> {
    let fkebank = lua.create_table()?;

    // fkebank.transfer(token_path, to_key, amount_cents [, description]) -> true or error
    {
        let fs = fs_service.clone();
        let svc = fkebank_service.clone();
        fkebank.set(
            "transfer",
            lua.create_function(
                move |lua, (token_path, to_key, amount_cents, description): (String, String, i64, Option<String>)| {
                    let ctx = lua
                        .app_data_ref::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    let vm_id = ctx.vm_id;
                    drop(ctx);

                    let token = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            let data = fs.read_file(vm_id, &token_path).await.ok();
                            data.and_then(|opt| opt.and_then(|(d, _)| String::from_utf8(d).ok()))
                        })
                    })
                    .ok_or_else(|| mlua::Error::runtime("Could not read token file"))?;
                    let token = token.trim().to_string();
                    if token.is_empty() {
                        return Err(mlua::Error::runtime("Token file is empty"));
                    }

                    let from_key = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            svc.get_key_by_token(&token).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?
                    .ok_or_else(|| mlua::Error::runtime("Invalid token"))?;

                    let _desc = description.as_deref().unwrap_or("Transfer");
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            svc.transfer(&from_key, &to_key, amount_cents, Some(&token)).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    Ok(Value::Boolean(true))
                },
            )?,
        )?;
    }

    // fkebank.key(token_path) -> account key string or nil (for display; company/USD key)
    {
        let fs = fs_service.clone();
        let svc = fkebank_service.clone();
        fkebank.set(
            "key",
            lua.create_function(move |lua, token_path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let token = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let data = fs.read_file(vm_id, &token_path).await.ok();
                        data.and_then(|opt| opt.and_then(|(d, _)| String::from_utf8(d).ok()))
                    })
                })
                .ok_or_else(|| mlua::Error::runtime("Could not read token file"))?;
                let token = token.trim().to_string();
                if token.is_empty() {
                    return Ok(Value::Nil);
                }

                let key = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_key_by_token(&token).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                match key {
                    Some(k) => Ok(Value::String(lua.create_string(&k)?)),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }

    // fkebank.balance(token_path) -> balance_cents or error
    {
        let fs = fs_service.clone();
        let svc = fkebank_service.clone();
        fkebank.set(
            "balance",
            lua.create_function(move |lua, token_path: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let token = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let data = fs.read_file(vm_id, &token_path).await.ok();
                        data.and_then(|opt| opt.and_then(|(d, _)| String::from_utf8(d).ok()))
                    })
                })
                .ok_or_else(|| mlua::Error::runtime("Could not read token file"))?;
                let token = token.trim().to_string();
                if token.is_empty() {
                    return Err(mlua::Error::runtime("Token file is empty"));
                }

                let key = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_key_by_token(&token).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?
                .ok_or_else(|| mlua::Error::runtime("Invalid token"))?;

                let balance = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_balance_by_key(&key).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(Value::Integer(balance))
            })?,
        )?;
    }

    // fkebank.history(token_path [, filter]) -> table of { from_key, to_key, amount, description, created_at_ms }; filter: "today" | "7d" | "30d" | ""
    {
        let fs = fs_service.clone();
        let svc = fkebank_service.clone();
        fkebank.set(
            "history",
            lua.create_function(move |lua, (token_path, filter): (String, Option<String>)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let token = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let data = fs.read_file(vm_id, &token_path).await.ok();
                        data.and_then(|opt| opt.and_then(|(d, _)| String::from_utf8(d).ok()))
                    })
                })
                .ok_or_else(|| mlua::Error::runtime("Could not read token file"))?;
                let token = token.trim().to_string();
                if token.is_empty() {
                    return Err(mlua::Error::runtime("Token file is empty"));
                }

                let key = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_key_by_token(&token).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?
                .ok_or_else(|| mlua::Error::runtime("Invalid token"))?;

                let filter = filter.unwrap_or_default();
                let rows = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.history_by_key(&key, Some(&token), &filter).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let tbl = lua.create_table()?;
                for (i, r) in rows.iter().enumerate() {
                    tbl.set(
                        i + 1,
                        tx_to_lua_table(
                            lua,
                            &r.id,
                            &r.from_key,
                            &r.to_key,
                            r.amount,
                            r.description.as_deref(),
                            r.created_at.timestamp_millis(),
                        )?,
                    )?;
                }
                Ok(Value::Table(tbl))
            })?,
        )?;
    }

    lua.globals().set("fkebank", fkebank)?;
    Ok(())
}
