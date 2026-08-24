mod commands;
mod overlay;
mod runtime;

use overlay::overlay_script;
pub(crate) use runtime::{
    dsh_entry, emit_progress, harness_package_manifest, healthy, hidden, install_runtime,
    marketplace_installed, node_bin, run_output, stop_harness_service, valid_runtime,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::runtime_status,
            commands::update_status,
            commands::launch_harness,
            commands::show_harness,
            commands::update_harness,
            commands::marketplace_status,
            commands::install_marketplace,
        ])
        .setup(|app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("DeepX failed to start");
}
