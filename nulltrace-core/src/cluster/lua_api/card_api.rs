//! Lua API for card invoices: create_invoice, pay_invoice, total_collected.
//! Only registered when wallet_card_service and card_invoice_service are available (e.g. card.null VM).

use crate::db::card_invoice_service::CardInvoiceService;
use mlua::{Lua, Result, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Register the `card` table.
pub fn register(lua: &Lua, card_invoice_service: Arc<CardInvoiceService>) -> Result<()> {
    let card = lua.create_table()?;

    // card.create_invoice(destination_key, amount_cents) -> invoice_id or error
    {
        let svc = card_invoice_service.clone();
        card.set(
            "create_invoice",
            lua.create_function(move |_lua, (destination_key, amount_cents): (String, i64)| {
                let invoice = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.create_invoice(&destination_key, amount_cents).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(Value::String(_lua.create_string(&invoice.id.to_string())?))
            })?,
        )?;
    }

    // card.pay_invoice(invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name) -> true or error
    {
        let svc = card_invoice_service.clone();
        card.set(
            "pay_invoice",
            lua.create_function(
                move |_lua,
                      (invoice_id, card_number, cvv, expiry_month, expiry_year, holder_name): (
                    String,
                    String,
                    String,
                    i32,
                    i32,
                    String,
                )| {
                    let id = Uuid::parse_str(&invoice_id)
                        .map_err(|_| mlua::Error::runtime("Invalid invoice id"))?;
                    tokio::task::block_in_place(|| {
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
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    Ok(Value::Boolean(true))
                },
            )?,
        )?;
    }

    // card.total_collected(destination_key) -> total_cents
    {
        let svc = card_invoice_service.clone();
        card.set(
            "total_collected",
            lua.create_function(move |_lua, destination_key: String| {
                let total = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        svc.get_total_collected(&destination_key).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(Value::Integer(total))
            })?,
        )?;
    }

    lua.globals().set("card", card)?;
    Ok(())
}
