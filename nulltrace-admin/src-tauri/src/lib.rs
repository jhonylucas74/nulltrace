mod grpc;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            grpc::admin_login,
            grpc::list_vms,
            grpc::get_cluster_stats,
            grpc::get_network_topology,
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
        .expect("error while running nulltrace-admin");
}
