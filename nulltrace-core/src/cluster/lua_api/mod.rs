pub mod context;
pub mod crypto_api;
pub mod fs_api;
pub mod fkebank_api;
pub mod http_api;
pub mod httpd_api;
pub mod incoming_money_api;
pub mod io_api;
pub mod mail_api;
pub mod net_api;
pub mod os_api;

use crate::db::crypto_wallet_service::CryptoWalletService;
use crate::db::email_account_service::EmailAccountService;
use crate::db::email_service::EmailService;
use crate::db::fkebank_account_service::FkebankAccountService;
use crate::db::fs_service::FsService;
use crate::db::user_service::UserService;
use crate::incoming_money_listener::IncomingMoneyListener;
use crate::mailbox_hub;
use mlua::{Lua, Result};
use std::sync::Arc;

/// Register all Lua APIs (fs, net, os, io, mail, optionally fkebank, crypto, incoming_money) and safe globals (load).
pub fn register_all(
    lua: &Lua,
    fs_service: Arc<FsService>,
    user_service: Arc<UserService>,
    email_service: Arc<EmailService>,
    email_account_service: Arc<EmailAccountService>,
    mailbox_hub: mailbox_hub::MailboxHub,
    fkebank_service: Option<Arc<FkebankAccountService>>,
    crypto_service: Option<Arc<CryptoWalletService>>,
    incoming_money_listener: Option<Arc<IncomingMoneyListener>>,
) -> Result<()> {
    fs_api::register(lua, fs_service.clone())?;
    net_api::register(lua)?;
    http_api::register(lua)?;
    httpd_api::register(lua, fs_service.clone())?;
    os_api::register(lua, user_service, fs_service.clone())?;
    io_api::register(lua)?;
    mail_api::register(lua, email_service, email_account_service, mailbox_hub)?;
    if let Some(ref fkebank) = fkebank_service {
        fkebank_api::register(lua, fs_service.clone(), fkebank.clone())?;
    }
    if let Some(ref crypto) = crypto_service {
        crypto_api::register(lua, fs_service.clone(), crypto.clone())?;
    }
    if let (Some(ref fkebank), Some(ref listener)) = (fkebank_service.as_ref(), incoming_money_listener.as_ref()) {
        incoming_money_api::register(lua, fs_service.clone(), Arc::clone(fkebank), Arc::clone(listener))?;
    }
    // Expose load(source, chunkname?, mode?) so /bin/lua can run user scripts. Sandbox may not expose it.
    let load_fn = lua.create_function(|lua, (source, chunkname, _mode): (String, Option<String>, Option<String>)| {
        let chunkname = chunkname.unwrap_or_else(|| "=(load)".to_string());
        let fn_result = lua.load(&source).set_name(&chunkname).into_function();
        match fn_result {
            Ok(f) => Ok(mlua::Value::Function(f)),
            Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
        }
    })?;
    lua.globals().set("load", load_fn)?;
    Ok(())
}
