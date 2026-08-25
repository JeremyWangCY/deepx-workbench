use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

mod commands;
mod overlay;
mod runtime;

use overlay::overlay_script;
pub(crate) use runtime::{
    configure_runtime_environment, dsh_entry, emit_progress, harness_package_manifest, healthy,
    hidden, install_runtime, marketplace_installed, marketplace_version, migrate_private_plugins,
    node_bin, pnpm_version, repair_marketplace_metadata, run_output_with_timeout, runtime_dir,
    seed_bundled_marketplace, set_update_channel, stop_harness_service, update_channel,
    update_runtime, valid_runtime, write_no_browser_patch, UpdateChannel,
};

fn activate_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            activate_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::runtime_status,
            commands::update_status,
            commands::launch_harness,
            commands::show_harness,
            commands::update_deepx,
            commands::initialize_harness,
            commands::update_harness,
            commands::marketplace_status,
            commands::install_marketplace,
            commands::get_update_channel,
            commands::select_update_channel,
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示 DeepX", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 DeepX", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::with_id("deepx-tray")
                .icon(
                    app.default_window_icon()
                        .expect("missing DeepX icon")
                        .clone(),
                )
                .tooltip("DeepX Workbench")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => activate_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        activate_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("DeepX failed to start");
}
