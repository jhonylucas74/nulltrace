//! Lua API for incoming_money: listen for new incoming transactions at a key.
//! listen_usd(token_path) for USD (resolves token to fkebank key); listen(to_key) for crypto.

use super::context::VmContext;
use crate::db::fkebank_account_service::FkebankAccountService;
use crate::db::fs_service::FsService;
use crate::incoming_money_listener::IncomingMoneyListener;
use mlua::{Lua, Result, Value};
use std::sync::Arc;

fn tx_to_lua_table(
    lua: &Lua,
    id: &uuid::Uuid,
    currency: &str,
    from_key: &str,
    to_key: &str,
    amount: i64,
    created_at_ms: i64,
) -> Result<Value> {
    let t = lua.create_table()?;
    t.set("id", id.to_string())?;
    t.set("currency", currency)?;
    t.set("from_key", from_key)?;
    t.set("to_key", to_key)?;
    t.set("amount", amount)?;
    t.set("created_at_ms", created_at_ms)?;
    Ok(Value::Table(t))
}

/// Register the `incoming_money` table. Requires vm_id in VmContext.
/// Only registered when fkebank and crypto are available (NPC VMs like money.null).
pub fn register(
    lua: &Lua,
    fs_service: Arc<FsService>,
    fkebank_service: Arc<FkebankAccountService>,
    listener: Arc<IncomingMoneyListener>,
) -> Result<()> {
    let incoming_money = lua.create_table()?;

    // incoming_money.listen_usd(token_path) -> nil or error
    {
        let fs = fs_service.clone();
        let svc = fkebank_service.clone();
        let listener = listener.clone();
        incoming_money.set(
            "listen_usd",
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

                listener.register(key, vm_id);
                Ok(Value::Nil)
            })?,
        )?;
    }

    // incoming_money.listen(to_key) -> nil (register for crypto address)
    {
        let listener = listener.clone();
        incoming_money.set(
            "listen",
            lua.create_function(move |lua, to_key: String| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let to_key = to_key.trim().to_string();
                if to_key.is_empty() {
                    return Err(mlua::Error::runtime("to_key cannot be empty"));
                }
                listener.register(to_key, vm_id);
                Ok(Value::Nil)
            })?,
        )?;
    }

    // incoming_money.recv() -> { id, currency, from_key, to_key, amount, created_at_ms } or nil (blocks)
    {
        let listener = listener.clone();
        incoming_money.set(
            "recv",
            lua.create_function(move |lua, ()| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let tx = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        listener.recv(vm_id).await
                    })
                });

                match tx {
                    Some(t) => Ok(tx_to_lua_table(
                        lua,
                        &t.id,
                        &t.currency,
                        &t.from_key,
                        &t.to_key,
                        t.amount,
                        t.created_at.timestamp_millis(),
                    )?),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }

    // incoming_money.try_recv() -> tx or nil (non-blocking; use in loop so VM can yield between ticks)
    {
        let listener = listener.clone();
        incoming_money.set(
            "try_recv",
            lua.create_function(move |lua, ()| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let tx = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        listener.try_recv(vm_id).await
                    })
                });

                match tx {
                    Some(t) => Ok(tx_to_lua_table(
                        lua,
                        &t.id,
                        &t.currency,
                        &t.from_key,
                        &t.to_key,
                        t.amount,
                        t.created_at.timestamp_millis(),
                    )?),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }

    lua.globals().set("incoming_money", incoming_money)?;
    Ok(())
}
