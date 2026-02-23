//! Lua API for crypto (BTC, ETH, SOL): create_wallet (generate keypair, write .priv to VM, register address),
//! transfer (read .priv from VM), history by address. No auth; identity is the public address.

use super::context::VmContext;
use crate::db::crypto_wallet_service::CryptoWalletService;
use crate::db::fs_service::FsService;
use crate::db::wallet_common::{generate_btc_address, generate_eth_address, generate_sol_address};
use mlua::{Lua, Result, Value};
use std::sync::Arc;
use uuid::Uuid;

fn tx_to_lua_table(lua: &Lua, from_key: &str, to_key: &str, amount: i64, description: Option<&str>, created_at: i64) -> Result<Value> {
    let t = lua.create_table()?;
    t.set("from_key", from_key)?;
    t.set("to_key", to_key)?;
    t.set("amount", amount)?;
    t.set("description", description.unwrap_or(""))?;
    t.set("created_at_ms", created_at)?;
    Ok(Value::Table(t))
}

fn generate_address(currency: &str) -> Option<String> {
    match currency {
        "BTC" => Some(generate_btc_address()),
        "ETH" => Some(generate_eth_address()),
        "SOL" => Some(generate_sol_address()),
        _ => None,
    }
}

/// Register the `crypto` table. Requires vm_id in VmContext.
pub fn register(lua: &Lua, fs_service: Arc<FsService>, crypto_service: Arc<CryptoWalletService>) -> Result<()> {
    let crypto = lua.create_table()?;

    // crypto.create_wallet(currency, output_dir) -> address (writes output_dir/btc.priv etc and registers address)
    {
        let fs = fs_service.clone();
        let svc = crypto_service.clone();
        crypto.set(
            "create_wallet",
            lua.create_function(move |lua, (currency, output_dir): (String, String)| {
                let ctx = lua
                    .app_data_ref::<VmContext>()
                    .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                let vm_id = ctx.vm_id;
                drop(ctx);

                let address = generate_address(currency.as_str())
                    .ok_or_else(|| mlua::Error::runtime("Invalid currency; use BTC, ETH, or SOL"))?;
                let priv_content = format!("{}", Uuid::new_v4().simple());

                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.register(&address, None, &currency).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let filename = format!("{}.priv", currency.to_lowercase());
                let path = if output_dir.ends_with('/') {
                    format!("{}{}", output_dir, filename)
                } else {
                    format!("{}/{}", output_dir, filename)
                };
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        fs.write_file(vm_id, &path, priv_content.as_bytes(), Some("text/plain"), "root").await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                Ok(Value::String(lua.create_string(&address)?))
            })?,
        )?;
    }

    // crypto.transfer(currency, from_address, priv_key_path, to_address, amount [, description])
    {
        let fs = fs_service.clone();
        let svc = crypto_service.clone();
        crypto.set(
            "transfer",
            lua.create_function(
                move |lua, (currency, from_address, priv_key_path, to_address, amount, description): (String, String, String, String, i64, Option<String>)| {
                    let ctx = lua
                        .app_data_ref::<VmContext>()
                        .ok_or_else(|| mlua::Error::runtime("No VM context"))?;
                    let vm_id = ctx.vm_id;
                    drop(ctx);

                    let (data, _) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            fs.read_file(vm_id, &priv_key_path).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?
                    .ok_or_else(|| mlua::Error::runtime("Could not read private key file"))?;
                    let priv_content = String::from_utf8(data).map_err(|_| mlua::Error::runtime("Private key file is not valid UTF-8"))?;
                    let priv_content = priv_content.trim().to_string();

                    let _desc = description.as_deref().unwrap_or("Transfer");
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            svc.transfer(&currency, &from_address, &to_address, amount, &priv_content).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    Ok(Value::Boolean(true))
                },
            )?,
        )?;
    }

    // crypto.history(address [, filter]) -> table of transactions; filter: "today" | "7d" | "30d" | ""
    {
        let svc = crypto_service.clone();
        crypto.set(
            "history",
            lua.create_function(move |lua, (address, filter): (String, Option<String>)| {
                let filter = filter.unwrap_or_default();
                let rows = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.history_by_address(&address, &filter).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;

                let tbl = lua.create_table()?;
                for (i, r) in rows.iter().enumerate() {
                    tbl.set(
                        i + 1,
                        tx_to_lua_table(
                            lua,
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

    // crypto.balance(address) -> amount
    {
        let svc = crypto_service.clone();
        crypto.set(
            "balance",
            lua.create_function(move |_lua, address: String| {
                let bal = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_balance(&address).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(Value::Integer(bal))
            })?,
        )?;
    }

    lua.globals().set("crypto", crypto)?;
    Ok(())
}
