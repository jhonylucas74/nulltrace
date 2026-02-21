//! Lua API for email: send, list, mark_read, move_to_folder, delete.
//! All operations require (address, token) and validate token via EmailAccountService.

use crate::db::email_account_service::EmailAccountService;
use crate::db::email_service::{EmailRecord, EmailService};
use crate::mailbox_hub;
use mlua::{Lua, Result, Value};
use std::sync::Arc;
use uuid::Uuid;

fn record_to_lua_table(lua: &Lua, r: &EmailRecord) -> Result<Value> {
    let t = lua.create_table()?;
    t.set("id", r.id.to_string())?;
    t.set("from_address", r.from_address.as_str())?;
    t.set("to_address", r.to_address.as_str())?;
    t.set("subject", r.subject.as_str())?;
    t.set("body", r.body.as_str())?;
    t.set("folder", r.folder.as_str())?;
    t.set("read", r.read)?;
    t.set("sent_at_ms", r.sent_at.timestamp_millis())?;
    Ok(Value::Table(t))
}

/// Register the `mail` table on the Lua state.
pub fn register(
    lua: &Lua,
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: mailbox_hub::MailboxHub,
) -> Result<()> {
    let mail = lua.create_table()?;

    // mail.send(from_address, token, to_address, subject, body [, opts]) -> true or error
    // opts can be { cc = "addr", bcc = "addr" } (optional). Call with 5 args for no cc/bcc.
    {
        let email_svc = email_service.clone();
        let account_svc = email_account_service.clone();
        let hub = mailbox_hub.clone();
        mail.set(
            "send",
            lua.create_function(
                move |_lua,
                      (from_address, token, to_address, subject, body, opts): (
                    String,
                    String,
                    String,
                    String,
                    String,
                    mlua::Value,
                )| {
                    let account_svc = account_svc.clone();
                    let email_svc = email_svc.clone();
                    let hub = hub.clone();
                    let (cc, bcc) = match &opts {
                        Value::Table(t) => {
                            let cc: Option<String> = t
                                .get("cc")
                                .ok()
                                .and_then(|v: Option<String>| v.filter(|s| !s.is_empty()));
                            let bcc: Option<String> = t
                                .get("bcc")
                                .ok()
                                .and_then(|v: Option<String>| v.filter(|s| !s.is_empty()));
                            (cc, bcc)
                        }
                        _ => (None, None),
                    };
                    let valid = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            account_svc.validate_token(&from_address, &token).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    if !valid {
                        return Err(mlua::Error::runtime("Invalid email token"));
                    }
                    let inbox_record = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            email_svc
                                .insert_email(&from_address, &to_address, &subject, &body, "inbox")
                                .await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    mailbox_hub::notify_new_email(&hub, &to_address, inbox_record);
                    if let Some(ref cc_addr) = cc {
                        if let Ok(cc_record) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                email_svc
                                    .insert_email(&from_address, cc_addr, &subject, &body, "inbox")
                                    .await
                            })
                        }) {
                            mailbox_hub::notify_new_email(&hub, cc_addr, cc_record);
                        }
                    }
                    if let Some(ref bcc_addr) = bcc {
                        if let Ok(bcc_record) = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async {
                                email_svc
                                    .insert_email(&from_address, bcc_addr, &subject, &body, "inbox")
                                    .await
                            })
                        }) {
                            mailbox_hub::notify_new_email(&hub, bcc_addr, bcc_record);
                        }
                    }
                    let _ = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            email_svc
                                .insert_email(
                                    &from_address,
                                    &from_address,
                                    &subject,
                                    &body,
                                    "sent",
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

    // mail.list(address, token, folder) -> table of emails or error
    {
        let email_svc = email_service.clone();
        let account_svc = email_account_service.clone();
        mail.set(
            "list",
            lua.create_function(move |lua, (address, token, folder): (String, String, String)| {
                let account_svc = account_svc.clone();
                let email_svc = email_svc.clone();
                let valid = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        account_svc.validate_token(&address, &token).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                if !valid {
                    return Err(mlua::Error::runtime("Invalid email token"));
                }
                let records = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        email_svc.list_emails(&address, &folder).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                let result = lua.create_table()?;
                for (i, r) in records.iter().enumerate() {
                    result.set(i + 1, record_to_lua_table(lua, r)?)?;
                }
                Ok(Value::Table(result))
            })?,
        )?;
    }

    // mail.mark_read(address, token, email_id, read) -> true or error
    {
        let email_svc = email_service.clone();
        let account_svc = email_account_service.clone();
        let hub = mailbox_hub.clone();
        mail.set(
            "mark_read",
            lua.create_function(
                move |_lua, (address, token, email_id, read): (String, String, String, bool)| {
                    let account_svc = account_svc.clone();
                    let email_svc = email_svc.clone();
                    let hub = hub.clone();
                    let valid = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            account_svc.validate_token(&address, &token).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    if !valid {
                        return Err(mlua::Error::runtime("Invalid email token"));
                    }
                    let id = Uuid::parse_str(&email_id)
                        .map_err(|_| mlua::Error::runtime("Invalid email_id"))?;
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            email_svc.mark_read(id, read).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    if let Ok(count) = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            email_svc.unread_count(&address).await
                        })
                    }) {
                        mailbox_hub::notify_unread_count(&hub, &address, count);
                    }
                    Ok(Value::Boolean(true))
                },
            )?,
        )?;
    }

    // mail.move_to_folder(address, token, email_id, folder) -> true or error
    {
        let email_svc = email_service.clone();
        let account_svc = email_account_service.clone();
        mail.set(
            "move_to_folder",
            lua.create_function(
                move |_lua, (address, token, email_id, folder): (String, String, String, String)| {
                    let account_svc = account_svc.clone();
                    let email_svc = email_svc.clone();
                    let valid = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            account_svc.validate_token(&address, &token).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    if !valid {
                        return Err(mlua::Error::runtime("Invalid email token"));
                    }
                    let id = Uuid::parse_str(&email_id)
                        .map_err(|_| mlua::Error::runtime("Invalid email_id"))?;
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            email_svc.move_to_folder(id, &folder).await
                        })
                    })
                    .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                    Ok(Value::Boolean(true))
                },
            )?,
        )?;
    }

    // mail.delete(address, token, email_id) -> true or error
    {
        let email_svc = email_service.clone();
        let account_svc = email_account_service.clone();
        mail.set(
            "delete",
            lua.create_function(move |_lua, (address, token, email_id): (String, String, String)| {
                let account_svc = account_svc.clone();
                let email_svc = email_svc.clone();
                let valid = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        account_svc.validate_token(&address, &token).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                if !valid {
                    return Err(mlua::Error::runtime("Invalid email token"));
                }
                let id = Uuid::parse_str(&email_id)
                    .map_err(|_| mlua::Error::runtime("Invalid email_id"))?;
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        email_svc.delete_email(id).await
                    })
                })
                .map_err(|e| mlua::Error::runtime(e.to_string()))?;
                Ok(Value::Boolean(true))
            })?,
        )?;
    }

    lua.globals().set("mail", mail)?;
    Ok(())
}
