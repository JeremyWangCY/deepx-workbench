use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

// ponytail: toolbar sits at z-index 20, below the plugin host layer (z-index 25,
// pointer-events:none), so any top-fixed plugin UI renders above it and stays
// clickable regardless of position; raises to 2147483647 would re-cover plugins.
const TOOLBAR_SCRIPT: &str = r###"(() => {
  if (window.__deepxToolbar && window.__deepxToolbar.isConnected) { return; }
  const css = '.deepx-toolbar{position:fixed;top:0;left:0;right:0;height:40px;z-index:20;display:flex;align-items:center;background:#f8f9fa;border-bottom:1px solid #e4e7eb;color:#202124;font:13px Segoe UI,system-ui,sans-serif;user-select:none}.deepx-toolbar-left{height:100%;display:flex;align-items:center;gap:4px;padding-left:8px}.deepx-toolbar-name{padding:0 8px;color:#5f6368}.deepx-toolbar-drag{height:100%;flex:1}.deepx-page-reload,.deepx-update-toggle{height:100%;border:0;border-radius:6px;background:transparent;color:#68717d;cursor:pointer;font:13px Segoe UI,system-ui,sans-serif}.deepx-page-reload{width:36px;font-size:20px}.deepx-page-reload:hover,.deepx-update-toggle:hover{background:#e9edf1;color:#202124}.deepx-page-reload:disabled{opacity:.4;cursor:default}.deepx-update-toggle{padding:0 12px}html{padding-top:40px!important;box-sizing:border-box!important}';
  const style = document.createElement('style');
  style.textContent = css;
  document.head.appendChild(style);
  const internals = window.__TAURI_INTERNALS__;
  const invoke = internals && internals.invoke ? internals.invoke.bind(internals) : null;
  const toolbar = document.createElement('header');
  toolbar.className = 'deepx-toolbar';
  toolbar.innerHTML = '<div class="deepx-toolbar-left"><button class="deepx-page-reload" title="刷新页面">↻</button><span class="deepx-toolbar-name">DeepX</span><button class="deepx-update-toggle" title="更新">更新</button></div><div class="deepx-toolbar-drag"></div>';
  document.body.appendChild(toolbar);
  window.__deepxToolbar = toolbar;
  toolbar.querySelector('.deepx-toolbar-drag').addEventListener('pointerdown', function (event) {
    if (event.button === 0 && invoke) { invoke('window_action', { action: 'start_dragging' }).catch(function () {}); }
  });
  const reloadButton = toolbar.querySelector('.deepx-page-reload');
  reloadButton.addEventListener('click', function () {
    if (reloadButton.disabled || !invoke) { return; }
    reloadButton.disabled = true;
    invoke('reload_harness').catch(function () { reloadButton.disabled = false; });
  });
  toolbar.querySelector('.deepx-update-toggle').addEventListener('click', function () {
    if (invoke) { invoke('open_update_window').catch(function () {}); }
  });
})();"###;

mod commands;
mod runtime;
pub(crate) use runtime::{
    configure_runtime_environment, dsh_entry, emit_progress, harness_package_manifest, healthy,
    hidden, install_runtime, marketplace_installed, marketplace_version, migrate_private_plugins,
    node_bin, repair_marketplace_metadata, run_output_with_timeout, runtime_dir,
    seed_bundled_marketplace, stop_harness_service, update_runtime, valid_runtime,
    write_no_browser_patch,
};

fn activate_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub(crate) fn open_update_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("update") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "update", WebviewUrl::App("update.html".into()))
        .title("DeepX 更新")
        .inner_size(480.0, 380.0)
        .resizable(false)
        .build()
        .map(|_| ())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            activate_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|webview, payload| {
            if webview.label() == "main"
                && payload.event() == PageLoadEvent::Finished
                && payload.url().host_str() == Some("127.0.0.1")
                && payload.url().port() == Some(3080)
            {
                let _ = webview.eval(TOOLBAR_SCRIPT);
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::window_action,
            commands::runtime_status,
            commands::update_status,
            commands::launch_harness,
            commands::show_harness,
            commands::reload_harness,
            commands::update_deepx,
            commands::initialize_harness,
            commands::update_harness,
            commands::marketplace_status,
            commands::install_marketplace,
            commands::open_update_window,
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示 DeepX", true, None::<&str>)?;
            let reload = MenuItem::with_id(app, "reload", "刷新页面", true, None::<&str>)?;
            let update = MenuItem::with_id(app, "update", "更新 DeepX", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 DeepX", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &reload, &update, &quit])?;

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
                    "update" => {
                        let _ = open_update_window(app);
                    }
                    "reload" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(url) = tauri::Url::parse("http://127.0.0.1:3080/") {
                                let _ = window.navigate(url);
                            }
                        }
                    }
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
