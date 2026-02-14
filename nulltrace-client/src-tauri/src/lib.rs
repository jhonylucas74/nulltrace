mod grpc;

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
fn get_app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(grpc::new_terminal_sessions())
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            run_luau,
            grpc::grpc_ping,
            grpc::grpc_login,
            grpc::grpc_refresh_token,
            grpc::grpc_disk_usage,
            grpc::grpc_get_process_list,
            grpc::grpc_sysinfo,
            grpc::grpc_restore_disk,
            grpc::grpc_get_ranking,
            grpc::grpc_get_player_profile,
            grpc::grpc_create_faction,
            grpc::grpc_leave_faction,
            grpc::grpc_get_home_path,
            grpc::grpc_list_fs,
            grpc::grpc_copy_path,
            grpc::grpc_move_path,
            grpc::grpc_rename_path,
            grpc::terminal_connect,
            grpc::terminal_send_stdin,
            grpc::terminal_send_interrupt,
            grpc::terminal_disconnect,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
