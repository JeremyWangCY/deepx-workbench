use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    webview::PageLoadEvent,
    AppHandle, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

const WINBAR_URL: &str = "http://127.0.0.1:3080/#winbar";

// The page-level WebView is unreliable at painting right-anchored UI in some
// environments, so the window controls live in their own tiny native window
// (OS-composited) docked to the top-right of the main window.
fn sync_winbar(main: &tauri::WebviewWindow) {
    let app = main.app_handle();
    let Some(winbar) = app.get_webview_window("winbar") else {
        return;
    };
    let minimized = main.is_minimized().unwrap_or(true);
    let visible = main.is_visible().unwrap_or(false);
    if minimized || !visible {
        let _ = winbar.hide();
        return;
    }
    if let (Ok(pos), Ok(size)) = (main.outer_position(), main.outer_size()) {
        let scale = main.scale_factor().unwrap_or(1.0);
        let width_px = (138.0 * scale) as i32;
        let _ = winbar.set_position(Position::Physical(PhysicalPosition::new(
            pos.x + size.width as i32 - width_px,
            pos.y,
        )));
    }
    let _ = winbar.show();
    let loaded = winbar
        .url()
        .map(|url| url.host_str() == Some("127.0.0.1") && url.port() == Some(3080))
        .unwrap_or(false);
    if !loaded {
        if let Ok(url) = tauri::Url::parse(WINBAR_URL) {
            let _ = winbar.navigate(url);
        }
    }
}

// ponytail: toolbar sits at z-index 20, below the plugin host layer (z-index 25,
// pointer-events:none), so any top-fixed plugin UI renders above it and stays
// clickable regardless of position; raises to 2147483647 would re-cover plugins.
const TOOLBAR_SCRIPT: &str = r###"(() => {
  if (window.location.hash === '#winbar') {
    try {
      const css2 = 'html,body{margin:0!important;padding:0!important;background:#f8f9fa!important;overflow:hidden!important}.deepx-winbar{position:fixed!important;top:0!important;left:0!important;right:0!important;bottom:0!important;display:flex!important;flex-direction:row!important;align-items:stretch!important;background:#f8f9fa!important;z-index:2147483647!important}.deepx-win{flex:1 1 0!important;height:100%!important;min-width:0!important;border:0!important;background:transparent!important;color:#5f6368!important;cursor:pointer!important;font:13px Segoe UI,system-ui,sans-serif!important;line-height:1!important;padding:0!important}.deepx-win:hover{background:#e9edf1!important;color:#202124!important}.deepx-win-close:hover{background:#e81123!important;color:#fff!important}';
      const st = document.createElement('style');
      st.textContent = css2;
      document.head.appendChild(st);
      const d = document.createElement('div');
      d.className = 'deepx-winbar';
      d.innerHTML = '<button class="deepx-win deepx-win-min" title="最小化">─</button><button class="deepx-win deepx-win-max" title="最大化">□</button><button class="deepx-win deepx-win-close" title="关闭">×</button>';
      document.body.appendChild(d);
      const inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke ? window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__) : null;
      const act = function (a) { if (inv) { inv('window_action', { action: a }).catch(function () {}); } };
      d.querySelector('.deepx-win-min').addEventListener('click', function () { act('minimize'); });
      d.querySelector('.deepx-win-max').addEventListener('click', function () { act('toggle_maximize'); });
      d.querySelector('.deepx-win-close').addEventListener('click', function () { act('close'); });
      window.__deepxWinbar = { mounted: true };
    } catch (error) { /* winbar best effort */ }
    return;
  }
  if (window.__deepxToolbar && window.__deepxToolbar.mounted) { return; }
  const css = '.deepx-toolbar{position:fixed!important;top:0!important;left:0!important;right:0!important;height:40px!important;z-index:2147483647!important;display:flex!important;flex-direction:row!important;align-items:center!important;background:#f8f9fa!important;border-bottom:1px solid #e4e7eb!important;color:#202124!important;font:13px Segoe UI,system-ui,sans-serif!important;user-select:none!important}.deepx-toolbar-left{height:100%!important;display:flex!important;align-items:center!important;gap:4px!important;padding-left:8px!important}.deepx-toolbar-name{padding:0 8px!important;color:#5f6368!important;font-weight:600!important}.deepx-toolbar-drag{height:100%!important;flex:1 1 auto!important;min-width:40px!important}.deepx-page-reload,.deepx-update-toggle{height:100%!important;border:0!important;border-radius:6px!important;background:transparent!important;color:#68717d!important;cursor:pointer!important;font:13px Segoe UI,system-ui,sans-serif!important}.deepx-page-reload{width:36px!important;font-size:20px!important}.deepx-page-reload:hover,.deepx-update-toggle:hover{background:#e9edf1!important;color:#202124!important}.deepx-page-reload:disabled{opacity:.4!important;cursor:default!important}.deepx-update-toggle{padding:0 12px!important}.deepx-panel{position:fixed!important;top:48px!important;left:8px!important;width:min(360px,calc(100vw - 24px))!important;padding:12px!important;border:1px solid #dfe3e8!important;border-radius:8px!important;background:#fff!important;box-shadow:0 10px 28px rgba(0,0,0,.19)!important;z-index:2147483646!important;font:13px Segoe UI,system-ui,sans-serif!important;color:#202124!important}.deepx-head{display:flex!important;align-items:center!important;justify-content:space-between!important}.deepx-title{font-weight:650!important}.deepx-refresh{width:24px!important;height:24px!important;padding:0!important;border:1px solid #dfe3e8!important;border-radius:5px!important;background:#fff!important;color:#5f6368!important;cursor:pointer!important;font-size:12px!important;line-height:1!important}.deepx-refresh:hover{color:#366cf6!important;border-color:#b9cbfa!important}.deepx-row{display:flex!important;justify-content:space-between!important;gap:12px!important;padding:4px 0!important;color:#5f6368!important}.deepx-btn{width:100%!important;margin-top:8px!important;padding:7px!important;border:0!important;border-radius:5px!important;background:#366cf6!important;color:#fff!important;cursor:pointer!important}.deepx-btn:disabled{opacity:.55!important;cursor:not-allowed!important}.deepx-track{height:5px!important;margin-top:9px!important;background:#e9edf2!important;border-radius:3px!important;overflow:hidden!important}.deepx-track i{display:block!important;height:100%!important;background:#366cf6!important;width:0!important;transition:width .2s!important}.deepx-status{color:#5f6368!important;font-size:11px!important;line-height:1.5!important;margin-top:6px!important;min-height:18px!important}.deepx-error{color:#c23d3d!important}html{padding-top:40px!important;box-sizing:border-box!important}';
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
  let toolbar = null;
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
  function fitHarnessBelowTitlebar() {
    if (!toolbar) { return false; }
    const all = document.querySelectorAll('*');
    let changed = false;
    for (let i = 0; i < all.length; i++) {
      const el = all[i];
      if (el === toolbar || toolbar.contains(el) || panel && panel.contains(el)) { continue; }
      const cs = getComputedStyle(el);
      if (cs.position !== 'fixed') { continue; }
      const r = el.getBoundingClientRect();
      if (r.width < 80 || r.height < 40) { continue; }
      const nearTop = Math.abs(r.top) < 2;
      if (!nearTop) { continue; }
      if (el.style.top === '40px') { continue; }
      el.style.top = '40px';
      if (r.height >= window.innerHeight * 0.9) { el.style.height = 'calc(100% - 40px)'; }
      changed = true;
    }
    return changed;
  }
  function startFitting() {
    let tries = 0;
    const retryFit = function () {
      if (tries >= 60) { return; }
      tries = tries + 1;
      if (fitHarnessBelowTitlebar()) { return; }
      setTimeout(retryFit, 500);
    };
    setTimeout(retryFit, 300);
    if (!window.MutationObserver) { return; }
    let pending = false;
    const observer = new MutationObserver(function () {
      if (pending) { return; }
      pending = true;
      setTimeout(function () { pending = false; fitHarnessBelowTitlebar(); }, 400);
    });
    const root = document.documentElement || document.body;
    if (root) { observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'style'] }); }
  }
  function mountToolbar() {
    if (toolbar || document.querySelector('.deepx-toolbar')) { return; }
    toolbar = document.createElement('header');
    toolbar.className = 'deepx-toolbar';
    toolbar.setAttribute('data-deepx-tb', '1');
    toolbar.innerHTML = '<div class="deepx-toolbar-left"><span class="deepx-toolbar-name">DeepX Workbench</span><button class="deepx-page-reload" title="刷新">↻</button><button class="deepx-update-toggle" title="更新">更新</button></div><div class="deepx-toolbar-drag"></div>';
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
    startFitting();
  }
  function win(action) { if (invoke) { invoke('window_action', { action: action }).catch(function () {}); } }
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
            if (webview.label() == "main" || webview.label() == "winbar")
                && payload.event() == PageLoadEvent::Finished
                && payload.url().host_str() == Some("127.0.0.1")
                && payload.url().port() == Some(3080)
            {
                let _ = webview.eval(TOOLBAR_SCRIPT);
            }
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    WindowEvent::Moved(_)
                    | WindowEvent::Resized(_)
                    | WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(main) = window.app_handle().get_webview_window("main") {
                            sync_winbar(&main);
                        }
                    }
                    WindowEvent::CloseRequested { api, .. } => {
                        if let Some(winbar) = window.app_handle().get_webview_window("winbar") {
                            let _ = winbar.hide();
                        }
                        let _ = window.hide();
                        api.prevent_close();
                    }
                    _ => {}
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
                WebviewWindowBuilder::new(
                    app,
                    "winbar",
                    WebviewUrl::External(tauri::Url::parse(WINBAR_URL).expect("winbar url")),
                )
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .maximizable(false)
                .minimizable(false)
                .shadow(false)
                .inner_size(138.0, 40.0)
                .visible(false)
                .build()?;
                sync_winbar(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("DeepX failed to start");
}
