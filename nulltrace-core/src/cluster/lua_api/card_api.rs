//! Lua API for card invoices: create_invoice, pay_invoice, total_collected.
//! Only registered when wallet_card_service and card_invoice_service are available (e.g. card.null VM).
//!
//! All functions return values instead of raising errors, so users never see internal Rust paths.
//! - create_invoice: (invoice_id) on success, (nil, "message") on failure
//! - pay_invoice: (true) on success, (false, "message") on failure
//! - total_collected: (total_cents) on success, (nil, "message") on failure

use crate::db::card_invoice_service::CardInvoiceService;
use mlua::{Lua, MultiValue, Result, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Register the `card` table.
pub fn register(lua: &Lua, card_invoice_service: Arc<CardInvoiceService>) -> Result<()> {
    let card = lua.create_table()?;

    // card.create_invoice(destination_key, amount_cents) -> (invoice_id) or (nil, "message")
    {
        let svc = card_invoice_service.clone();
        card.set(
            "create_invoice",
            lua.create_function(move |lua, (destination_key, amount_cents): (String, i64)| {
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.create_invoice(&destination_key, amount_cents).await
                    })
                });
                match result {
                    Ok(invoice) => {
                        let id_val = Value::String(lua.create_string(&invoice.id.to_string())?);
                        Ok(MultiValue::from_vec(vec![id_val]))
                    }
                    Err(e) => {
                        let err_val = Value::String(lua.create_string(&e.to_string())?);
                        Ok(MultiValue::from_vec(vec![Value::Nil, err_val]))
                    }
                }
            })?,
        )?;
    }

    // card.pay_invoice(invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name) -> (true) or (false, "message")
    {
        let svc = card_invoice_service.clone();
        card.set(
            "pay_invoice",
            lua.create_function(
                move |lua,
                      (invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name): (
                    String,
                    String,
                    String,
                    i32,
                    i32,
                    String,
                )| {
                    let id = match Uuid::parse_str(&invoice_id) {
                        Ok(u) => u,
                        Err(_) => {
                            let err_val = Value::String(lua.create_string("Invalid invoice id")?);
                            return Ok(MultiValue::from_vec(vec![Value::Boolean(false), err_val]));
                        }
                    };
                    let result = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            svc.pay_invoice(
                                id,
                                &card_number,
                                &cvv,
                                expiry_month,
                                expiry_year,
                                &holder_name,
                            )
                            .await
                        })
                    });
                    match result {
                        Ok(()) => Ok(MultiValue::from_vec(vec![Value::Boolean(true)])),
                        Err(e) => {
                            let err_val = Value::String(lua.create_string(&e.to_string())?);
                            Ok(MultiValue::from_vec(vec![Value::Boolean(false), err_val]))
                        }
                    }
                },
            )?,
        )?;
    }

    // card.total_collected(destination_key) -> (total_cents) or (nil, "message")
    {
        let svc = card_invoice_service.clone();
        card.set(
            "total_collected",
            lua.create_function(move |lua, destination_key: String| {
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_total_collected(&destination_key).await
                    })
                });
                match result {
                    Ok(total) => Ok(MultiValue::from_vec(vec![Value::Integer(total)])),
                    Err(e) => {
                        let err_val = Value::String(lua.create_string(&e.to_string())?);
                        Ok(MultiValue::from_vec(vec![Value::Nil, err_val]))
                    }
                }
            })?,
        )?;
    }

    lua.globals().set("card", card)?;
    Ok(())
}
