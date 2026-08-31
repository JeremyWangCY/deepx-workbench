use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    AppHandle, Manager, WindowEvent,
};

// ponytail: toolbar sits at z-index 20, below the plugin host layer (z-index 25,
// pointer-events:none), so any top-fixed plugin UI renders above it and stays
// clickable regardless of position; raises to 2147483647 would re-cover plugins.
const TOOLBAR_SCRIPT: &str = r###"(() => {
  if (window.__deepxToolbar && window.__deepxToolbar.mounted) { return; }
  const css = '.deepx-toolbar{position:fixed;top:0;left:0;right:0;height:40px;z-index:20;display:flex;align-items:center;background:#f8f9fa;border-bottom:1px solid #e4e7eb;color:#202124;font:13px Segoe UI,system-ui,sans-serif;user-select:none}.deepx-toolbar-left{height:100%;display:flex;align-items:center;gap:2px;padding-left:4px}.deepx-toolbar-name{padding:0 8px;color:#5f6368;font-weight:600}.deepx-toolbar-drag{height:100%;flex:1;min-width:40px}.deepx-page-reload,.deepx-update-toggle{height:100%;border:0;border-radius:6px;background:transparent;color:#68717d;cursor:pointer;font:13px Segoe UI,system-ui,sans-serif}.deepx-page-reload{width:34px;font-size:20px}.deepx-page-reload:hover,.deepx-update-toggle:hover{background:#e9edf1;color:#202124}.deepx-page-reload:disabled{opacity:.4;cursor:default}.deepx-update-toggle{padding:0 10px}.deepx-win{width:38px;height:100%;border:0;background:transparent;color:#68717d;cursor:pointer;font:12px Segoe UI,system-ui,sans-serif;line-height:1}.deepx-win:hover{background:#e9edf1;color:#202124}.deepx-win-close:hover{background:#e81123;color:#fff}.deepx-panel{position:fixed;top:48px;left:8px;width:min(360px,calc(100vw - 24px));padding:12px;border:1px solid #dfe3e8;border-radius:8px;background:#fff;box-shadow:0 10px 28px rgba(0,0,0,.19);z-index:30;font:13px Segoe UI,system-ui,sans-serif;color:#202124}.deepx-head{display:flex;align-items:center;justify-content:space-between}.deepx-title{font-weight:650}.deepx-refresh{width:24px;height:24px;padding:0;border:1px solid #dfe3e8;border-radius:5px;background:#fff;color:#5f6368;cursor:pointer;font-size:12px;line-height:1}.deepx-refresh:hover{color:#366cf6;border-color:#b9cbfa}.deepx-row{display:flex;justify-content:space-between;gap:12px;padding:4px 0;color:#5f6368}.deepx-btn{width:100%;margin-top:8px;padding:7px;border:0;border-radius:5px;background:#366cf6;color:#fff;cursor:pointer}.deepx-btn:disabled{opacity:.55;cursor:not-allowed}.deepx-track{height:5px;margin-top:9px;background:#e9edf2;border-radius:3px;overflow:hidden}.deepx-track i{display:block;height:100%;background:#366cf6;width:0;transition:width .2s}.deepx-status{color:#5f6368;font-size:11px;line-height:1.5;margin-top:6px;min-height:18px}.deepx-error{color:#c23d3d}html{padding-top:40px!important;box-sizing:border-box!important}';
  const style = document.createElement('style');
  style.textContent = css;
  document.head.appendChild(style);
  const internals = window.__TAURI_INTERNALS__;
  const invoke = internals && internals.invoke ? internals.invoke.bind(internals) : null;
  let panel = null;
  let busy = false;
  let progressValue = 0;
  let statusMessage = '';
  let statusError = false;
  let updateStatus = null;
  let statusRequest = null;
  function win(action) { if (invoke) { invoke('window_action', { action: action }).catch(function () {}); } }
  function fmtBytes(n) {
    if (n == null) { return '--'; }
    const mb = n / 1048576;
    if (mb >= 1024) { return (mb / 1024).toFixed(2) + ' GB'; }
    if (mb >= 1) { return mb.toFixed(1) + ' MB'; }
    return Math.round(n / 1024) + ' KB';
  }
  function fmtSpeed(bps) {
    if (!bps || bps <= 0) { return '--'; }
    const mbps = bps / 1048576;
    if (mbps >= 1) { return mbps.toFixed(1) + ' MB/s'; }
    return Math.max(1, Math.round(bps / 1024)) + ' KB/s';
  }
  function fmtEta(remainingBytes, bps) {
    if (!bps || bps <= 0 || remainingBytes <= 0) { return '--'; }
    const seconds = remainingBytes / bps;
    if (seconds < 90) { return '剩余 ' + Math.max(1, Math.round(seconds)) + ' 秒'; }
    if (seconds < 5400) { return '剩余 ' + Math.round(seconds / 60) + ' 分钟'; }
    return '剩余 ' + (seconds / 3600).toFixed(1) + ' 小时';
  }
  function setStatus(text, isError) {
    statusMessage = text;
    statusError = !!isError;
    const output = panel && panel.querySelector('.deepx-status');
    if (output) { output.textContent = statusMessage; output.classList.toggle('deepx-error', statusError); }
  }
  function setProgress(value) {
    progressValue = Math.max(0, Math.min(100, Number(value) || 0));
    const indicator = panel && panel.querySelector('.deepx-track i');
    if (indicator) { indicator.style.width = progressValue + '%'; }
  }
  function setBusy(next, message) {
    busy = next;
    if (panel) { panel.querySelectorAll('.deepx-btn').forEach(function (button) { button.disabled = busy; }); }
    setStatus(message, false);
  }
  function versionText(status) {
    if (!status) { return '未检查'; }
    return '当前 ' + (status.current || '未安装') + ' · 最新 ' + (status.latest || '未知');
  }
  function actionLabel(status, target) {
    if (!status || !status.current) { return '安装 ' + target; }
    return status.update_available ? '更新 ' + target : '';
  }
  function applyStatus(status) {
    updateStatus = status;
    if (!panel) { return; }
    panel.querySelector('.deepx-app-version').textContent = versionText(status && status.deepx);
    panel.querySelector('.deepx-version').textContent = versionText(status && status.harness);
    panel.querySelector('.deepx-market-version').textContent = versionText(status && status.marketplace);
    renderActions();
  }
  function refreshStatus() {
    if (!invoke) { return Promise.resolve(); }
    if (statusRequest) { return statusRequest; }
    statusRequest = invoke('update_status').then(applyStatus).finally(function () { statusRequest = null; });
    return statusRequest;
  }
  function renderActions() {
    const actions = panel && panel.querySelector('.deepx-actions');
    if (!actions) { return; }
    actions.innerHTML = '';
    const definitions = [['deepx','deepx-app-update','update_deepx','DeepX'],['harness','deepx-update','update_harness','Harness'],['marketplace','deepx-market-update','install_marketplace','插件市场']];
    definitions.forEach(function (item) {
      const key = item[0];
      const command = item[2];
      const target = item[3];
      const label = updateStatus ? actionLabel(updateStatus[key], target) : '';
      if (!label) { return; }
      const button = document.createElement('button');
      button.className = 'deepx-btn ' + item[1];
      button.textContent = label;
      button.disabled = busy;
      button.onclick = function () { runAction(command, key); };
      actions.appendChild(button);
    });
  }
  async function runAction(command, key) {
    if (busy || !invoke) { return; }
    try {
      if (key === 'deepx') { setBusy(true, '正在下载 DeepX 更新...'); }
      else if (key === 'harness') { setBusy(true, '正在更新 Harness...'); }
      else { setBusy(true, '正在准备插件市场...'); }
      await invoke(command);
      if (key === 'deepx') { setBusy(false, 'DeepX 更新安装器已启动'); }
      else if (key === 'harness') { setBusy(false, 'Harness 已更新'); }
      else { setBusy(false, '插件市场已更新'); }
      if (updateStatus && updateStatus[key]) {
        updateStatus[key].current = updateStatus[key].latest || updateStatus[key].current || '已安装';
        updateStatus[key].update_available = false;
        applyStatus(updateStatus);
      }
    } catch (error) {
      setBusy(false, String(error), true);
      setProgress(0);
    }
  }
  function drawPanel() {
    if (!panel) { return; }
    panel.innerHTML = '<div class="deepx-head"><span class="deepx-title">DeepX</span><button class="deepx-refresh" title="刷新状态">↻</button></div><div class="deepx-row"><span>DeepX</span><span class="deepx-app-version">未检查</span></div><div class="deepx-row"><span>Harness</span><span class="deepx-version">未检查</span></div><div class="deepx-row"><span>插件市场</span><span class="deepx-market-version">未检查</span></div><div class="deepx-actions"></div><div class="deepx-track"><i></i></div><div class="deepx-status"></div>';
    setProgress(progressValue);
    setStatus(statusMessage, statusError);
    applyStatus(updateStatus || { deepx: null, harness: null, marketplace: null });
    const refreshButton = panel.querySelector('.deepx-refresh');
    refreshButton.onclick = async function () {
      if (busy || refreshButton.disabled) { return; }
      refreshButton.disabled = true;
      setStatus('正在刷新状态...', false);
      try { await refreshStatus(); setStatus('状态已刷新', false); }
      catch (error) { setStatus(String(error), true); }
      finally { refreshButton.disabled = false; }
    };
  }
  function togglePanel() {
    if (!invoke) { return; }
    if (panel) { panel.remove(); panel = null; return; }
    panel = document.createElement('div');
    panel.className = 'deepx-panel';
    document.body.appendChild(panel);
    drawPanel();
    refreshStatus().catch(function () {});
  }
  function mountToolbar() {
    if (document.querySelector('.deepx-toolbar')) { return; }
    const toolbar = document.createElement('header');
    toolbar.className = 'deepx-toolbar';
    toolbar.innerHTML = '<div class="deepx-toolbar-left"><button class="deepx-win deepx-win-min" title="最小化">─</button><button class="deepx-win deepx-win-max" title="最大化">□</button><button class="deepx-win deepx-win-close" title="关闭">✕</button><span class="deepx-toolbar-name">DeepX Workbench</span><button class="deepx-page-reload" title="刷新">↻</button><button class="deepx-update-toggle" title="更新">更新</button></div><div class="deepx-toolbar-drag"></div>';
    document.body.appendChild(toolbar);
    const drag = toolbar.querySelector('.deepx-toolbar-drag');
    drag.addEventListener('pointerdown', function (event) {
      if (event.button === 0 && invoke) { invoke('window_action', { action: 'start_dragging' }).catch(function () {}); }
    });
    drag.addEventListener('dblclick', function () { win('toggle_maximize'); });
    const reloadButton = toolbar.querySelector('.deepx-page-reload');
    reloadButton.addEventListener('click', function () {
      if (reloadButton.disabled || !invoke) { return; }
      reloadButton.disabled = true;
      invoke('reload_harness').catch(function () { reloadButton.disabled = false; });
    });
    toolbar.querySelector('.deepx-update-toggle').addEventListener('click', togglePanel);
    toolbar.querySelector('.deepx-win-min').addEventListener('click', function () { win('minimize'); });
    toolbar.querySelector('.deepx-win-max').addEventListener('click', function () { win('toggle_maximize'); });
    toolbar.querySelector('.deepx-win-close').addEventListener('click', function () { win('close'); });
  }
  if (internals && internals.transformCallback && internals.invoke) {
    internals.invoke('plugin:event|listen', { event: 'deepx-update-progress', handler: internals.transformCallback(function (payload) {
      const p = payload || {};
      if (p.percent != null) { setProgress(p.percent); }
      if (panel && p.downloaded != null) {
        setStatus(fmtBytes(p.downloaded) + ' / ' + fmtBytes(p.total) + ' · ' + fmtSpeed(p.speed) + ' · ' + fmtEta(p.total - p.downloaded, p.speed), false);
      }
    }) }).catch(function () {});
    internals.invoke('plugin:event|listen', { event: 'runtime-progress', handler: internals.transformCallback(function (payload) {
      const p = payload || {};
      if (p.percentage != null) { setProgress(p.percentage); }
      if (p.detail != null) { setStatus(String(p.detail), !!(p.error)); }
    }) }).catch(function () {});
  }
  window.__deepxToolbar = { mounted: true, togglePanel: togglePanel };
  mountToolbar();
  refreshStatus().catch(function () {});
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
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.eval(
                                r#"window.__deepxToolbar && window.__deepxToolbar.togglePanel && window.__deepxToolbar.togglePanel()"#,
                            );
                        }
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
