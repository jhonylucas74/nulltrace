mod grpc;
mod ntml_html;
mod ntml_runtime;

use mlua::{Lua, VmState};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Result of executing a Luau script in the sandbox.
#[derive(serde::Serialize)]
struct LuauResult {
    success: bool,
    output: Vec<String>,
    error: Option<String>,
}

/// Tauri command: run Luau code in a sandboxed VM and capture print() output.
/// Uses the same mlua library as nulltrace-core with sandbox(true).
#[tauri::command]
fn run_luau(code: String) -> LuauResult {
    let lua = Lua::new();
    let _ = lua.sandbox(true);

    // Capture print() output into a shared vector
    let output = Arc::new(Mutex::new(Vec::<String>::new()));
    let output_clone = output.clone();

    let print_fn = lua
        .create_function(move |_, args: mlua::Variadic<String>| {
            let line = args.join("\t");
            output_clone.lock().unwrap().push(line);
            Ok(())
        })
        .unwrap();

    lua.globals().set("print", print_fn).unwrap();

    // Interrupt to prevent infinite loops: stop after 1M instructions
    let count = AtomicU64::new(0);
    lua.set_interrupt(move |_| {
        if count.fetch_add(1, Ordering::Relaxed) > 1_000_000 {
            return Ok(VmState::Yield);
        }
        Ok(VmState::Continue)
    });

    match lua.load(&code).exec() {
        Ok(_) => LuauResult {
            success: true,
            output: output.lock().unwrap().clone(),
            error: None,
        },
        Err(e) => LuauResult {
            success: false,
            output: output.lock().unwrap().clone(),
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
fn ntml_to_html(
    yaml: String,
    imports: Option<Vec<NtmlImportPayload>>,
    markdown_contents: Option<Vec<NtmlMarkdownPayload>>,
    base_url: Option<String>,
) -> Result<String, String> {
    let imp = imports.unwrap_or_default();
    let ntml_imports: Vec<ntml_html::NtmlImport> = imp
        .into_iter()
        .map(|i| ntml_html::NtmlImport {
            alias: i.alias,
            content: i.content,
        })
        .collect();
    let md_map: std::collections::HashMap<String, String> = markdown_contents
        .unwrap_or_default()
        .into_iter()
        .map(|m| (m.src, m.content))
        .collect();
    let base = base_url.as_deref();
    ntml_html::ntml_to_html_with_imports_and_patches(&yaml, &ntml_imports, &md_map, &[], base)
}

#[derive(serde::Deserialize)]
struct NtmlImportPayload {
    alias: String,
    content: String,
}

#[derive(serde::Deserialize)]
struct NtmlMarkdownPayload {
    src: String,
    content: String,
}

#[tauri::command]
fn ntml_create_tab_state(
    tab_id: String,
    base_url: String,
    script_sources: Vec<ntml_runtime::ScriptSource>,
    component_yaml: String,
    imports: Vec<ntml_runtime::TabImport>,
    markdown_contents: Option<Vec<NtmlMarkdownPayload>>,
    user_id: String,
    store: tauri::State<ntml_runtime::TabStateStore>,
    storage: tauri::State<ntml_runtime::BrowserStorageStore>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let md: Vec<(String, String)> = markdown_contents
        .unwrap_or_default()
        .into_iter()
        .map(|m| (m.src, m.content))
        .collect();
    ntml_runtime::create_tab_state(
        &store,
        tab_id,
        base_url,
        script_sources,
        component_yaml,
        imports,
        md,
        (*storage).clone(),
        user_id,
        app,
    )
}

#[tauri::command]
fn ntml_run_handler(
    tab_id: String,
    action: String,
    form_values: Option<std::collections::HashMap<String, String>>,
    event_data: Option<std::collections::HashMap<String, String>>,
    store: tauri::State<ntml_runtime::TabStateStore>,
) -> Result<ntml_run_handler_result::NtmlRunHandlerResult, String> {
    let (patches, print_output) =
        ntml_runtime::run_handler(&store, &tab_id, &action, form_values, event_data)?;
    let html = ntml_runtime::render_with_accumulated_patches(&store, &tab_id, &patches)?;
    Ok(ntml_run_handler_result::NtmlRunHandlerResult { patches, html, print_output })
}

mod ntml_run_handler_result {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct NtmlRunHandlerResult {
        pub patches: Vec<crate::ntml_runtime::Patch>,
        pub html: String,
        pub print_output: Vec<String>,
    }
}

#[tauri::command]
fn ntml_close_tab(tab_id: String, store: tauri::State<ntml_runtime::TabStateStore>) {
    ntml_runtime::close_tab(&store, &tab_id);
}

#[tauri::command]
fn ntml_get_head_resources(yaml: String) -> Result<ntml_runtime::NtmlHeadResources, String> {
    ntml_runtime::get_head_resources(&yaml)
}

// ── Browser LocalStorage API ──────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize)]
struct StorageKvEntry {
    key: String,
    value: String,
}

#[tauri::command]
fn browser_storage_get_all(
    user_id: String,
    origin: String,
    store: tauri::State<ntml_runtime::BrowserStorageStore>,
    app: tauri::AppHandle,
) -> Vec<StorageKvEntry> {
    // Lazy-load from disk if this origin hasn't been accessed this session yet
    ntml_runtime::ensure_origin_loaded(&app, &user_id, &origin, &store);
    let key = ntml_runtime::storage_composite_key(&user_id, &origin);
    store
        .get(&key)
        .map(|m| {
            m.iter()
                .map(|(k, v)| StorageKvEntry { key: k.clone(), value: v.clone() })
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
fn browser_storage_set(
    user_id: String,
    origin: String,
    key: String,
    value: String,
    store: tauri::State<ntml_runtime::BrowserStorageStore>,
    app: tauri::AppHandle,
) {
    let ck = ntml_runtime::storage_composite_key(&user_id, &origin);
    store.entry(ck).or_default().insert(key, value);
    ntml_runtime::persist_origin_storage(&app, &user_id, &origin, &store);
}

#[tauri::command]
fn browser_storage_delete(
    user_id: String,
    origin: String,
    key: String,
    store: tauri::State<ntml_runtime::BrowserStorageStore>,
    app: tauri::AppHandle,
) {
    let ck = ntml_runtime::storage_composite_key(&user_id, &origin);
    if let Some(mut m) = store.get_mut(&ck) {
        m.remove(&key);
    }
    ntml_runtime::persist_origin_storage(&app, &user_id, &origin, &store);
}

#[tauri::command]
fn browser_storage_clear(
    user_id: String,
    origin: String,
    store: tauri::State<ntml_runtime::BrowserStorageStore>,
    app: tauri::AppHandle,
) {
    let ck = ntml_runtime::storage_composite_key(&user_id, &origin);
    store.remove(&ck);
    if let Some(path) = ntml_runtime::storage_file_path(&app, &user_id, &origin) {
        let _ = std::fs::remove_file(&path);
    }
}

#[tauri::command]
fn ntml_eval_lua(
    tab_id: String,
    code: String,
    store: tauri::State<ntml_runtime::TabStateStore>,
) -> ntml_runtime::EvalLuaResult {
    ntml_runtime::eval_lua(&store, &tab_id, &code)
}

#[tauri::command]
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(grpc::new_terminal_sessions())
        .manage(grpc::new_process_spy_state())
        .manage(grpc::new_mailbox_connections())
        .manage(ntml_runtime::new_tab_state_store())
        .manage(ntml_runtime::new_browser_storage_store())
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            ntml_to_html,
            ntml_create_tab_state,
            ntml_run_handler,
            ntml_close_tab,
            ntml_get_head_resources,
            ntml_eval_lua,
            run_luau,
            browser_storage_get_all,
            browser_storage_set,
            browser_storage_delete,
            browser_storage_clear,
            grpc::grpc_ping,
            grpc::grpc_login,
            grpc::grpc_refresh_token,
            grpc::grpc_disk_usage,
            grpc::grpc_get_process_list,
            grpc::grpc_sysinfo,
            grpc::grpc_restore_disk,
            grpc::grpc_get_ranking,
            grpc::grpc_get_player_profile,
            grpc::grpc_set_preferred_theme,
            grpc::grpc_set_shortcuts,
            grpc::grpc_create_faction,
            grpc::grpc_leave_faction,
            grpc::grpc_get_home_path,
            grpc::grpc_list_fs,
            grpc::grpc_copy_path,
            grpc::grpc_move_path,
            grpc::grpc_rename_path,
            grpc::grpc_create_folder,
            grpc::grpc_run_process,
            grpc::grpc_write_file,
            grpc::grpc_read_file,
            grpc::grpc_empty_trash,
            grpc::grpc_get_installed_store_apps,
            grpc::grpc_install_store_app,
            grpc::grpc_uninstall_store_app,
            grpc::terminal_connect,
            grpc::code_run_connect,
            grpc::terminal_send_stdin,
            grpc::terminal_send_interrupt,
            grpc::terminal_disconnect,
            grpc::process_spy_connect,
            grpc::process_spy_subscribe,
            grpc::process_spy_unsubscribe,
            grpc::process_spy_stdin,
            grpc::process_spy_spawn_lua_script,
            grpc::process_spy_kill_process,
            grpc::process_spy_disconnect,
            grpc::grpc_get_emails,
            grpc::grpc_send_email,
            grpc::grpc_mark_email_read,
            grpc::grpc_move_email,
            grpc::grpc_delete_email,
            grpc::mailbox_connect,
            grpc::mailbox_disconnect,
            grpc::grpc_get_wallet_balances,
            grpc::grpc_get_wallet_transactions,
            grpc::grpc_get_wallet_keys,
            grpc::grpc_transfer_funds,
            grpc::grpc_resolve_transfer_key,
            grpc::grpc_convert_funds,
            grpc::grpc_get_wallet_cards,
            grpc::grpc_create_wallet_card,
            grpc::grpc_delete_wallet_card,
            grpc::grpc_get_card_transactions,
            grpc::grpc_get_card_statement,
            grpc::grpc_pay_card_bill,
        ])
        .setup(|app| {
            if std::env::var_os("TAURI_OPEN_DEVTOOLS").is_some() {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
