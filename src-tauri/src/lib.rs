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
  if (location.hostname !== '127.0.0.1' || location.port !== '3080') { return; }
  var liveInvoke = getInvoke();
  var probe = function (br) {
    try {
      var tbs = document.querySelectorAll('header.deepx-toolbar[data-deepx-tb]');
      var el = null;
      if (tbs.length) { var b0 = tbs[0].querySelector('.deepx-update-toggle'); if (b0) { var r = b0.getBoundingClientRect(); el = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2); } }
      var diag = JSON.stringify({ br: br, in: !!window.__TAURI_INTERNALS__, pi: !!liveInvoke, n: tbs.length, conn: tbs.length ? !!tbs[0].isConnected : false, own: !!(window.__deepxToolbar && window.__deepxToolbar.toolbar && tbs.length && window.__deepxToolbar.toolbar === tbs[0]), hasI: !!(window.__deepxToolbar && window.__deepxToolbar.hasInvoke), hit: !!(el && tbs.length && tbs[0].contains(el)) });
      if (liveInvoke) { liveInvoke('toolbar_probe', { diag: diag }).catch(function () {}); }
    } catch (e) {}
  };
  const existing = document.querySelector('header.deepx-toolbar[data-deepx-tb]');
  if (existing && existing.isConnected && window.__deepxToolbar && window.__deepxToolbar.hasInvoke && window.__deepxToolbar.toolbar === existing && liveInvoke) {
    probe(1);
    window.__deepxToolbar.remount();
    return;
  }
  if (!liveInvoke) { return; }
  var stales = document.querySelectorAll('header.deepx-toolbar[data-deepx-tb]');
  for (var si = 0; si < stales.length; si++) { stales[si].remove(); }
  const staleStyle = document.getElementById('deepx-toolbar-style');
  if (staleStyle) { staleStyle.remove(); }
  window.__deepxToolbar = null;
  const css = ''
    + '.deepx-toolbar{position:fixed!important;top:0!important;left:0!important;right:0!important;height:40px!important;z-index:2147483647!important;display:flex!important;flex-direction:row!important;align-items:center!important;background:#f8f9fa!important;border-bottom:1px solid #e4e7eb!important;color:#202124!important;font:13px Segoe UI,system-ui,sans-serif!important;user-select:none!important}.deepx-toolbar-left{height:100%!important;display:flex!important;align-items:center!important;gap:4px!important;padding-left:8px!important}.deepx-toolbar-name{padding:0 8px!important;color:#5f6368!important;font-weight:600!important}.deepx-toolbar-drag{height:100%!important;flex:1 1 auto!important;min-width:40px!important}'
    + '.deepx-page-reload,.deepx-update-toggle{height:100%!important;border:0!important;border-radius:6px!important;background:transparent!important;color:#68717d!important;cursor:pointer!important;font:13px Segoe UI,system-ui,sans-serif!important}.deepx-page-reload{width:36px!important;font-size:20px!important}.deepx-page-reload:hover,.deepx-update-toggle:hover{background:#e9edf1!important;color:#202124!important}.deepx-page-reload:disabled{opacity:.4!important;cursor:default!important}.deepx-update-toggle{padding:0 12px!important}.deepx-toolbar-win{height:100%!important;display:flex!important;flex-direction:row!important;align-items:stretch!important;margin-left:6px!important;border-left:1px solid #e4e7eb!important}'
    + '.deepx-win{width:44px!important;height:100%!important;border:0!important;background:transparent!important;color:#5f6368!important;cursor:pointer!important;font:14px Segoe UI,system-ui,sans-serif!important;line-height:1!important;padding:0!important;display:flex!important;align-items:center!important;justify-content:center!important}.deepx-win:hover{background:#e9edf1!important;color:#202124!important}.deepx-win-close:hover{background:#e81123!important;color:#fff!important}'
    + '.deepx-panel{position:fixed!important;top:48px!important;left:8px!important;width:min(360px,calc(100vw - 24px))!important;padding:12px!important;border:1px solid #dfe3e8!important;border-radius:8px!important;background:#fff!important;box-shadow:0 10px 28px rgba(0,0,0,.19)!important;z-index:2147483646!important;font:13px Segoe UI,system-ui,sans-serif!important;color:#202124!important}.deepx-head{display:flex!important;align-items:center!important;justify-content:space-between!important}.deepx-title{font-weight:650!important}.deepx-refresh{width:24px!important;height:24px!important;padding:0!important;border:1px solid #dfe3e8!important;border-radius:5px!important;background:#fff!important;color:#5f6368!important;cursor:pointer!important;font-size:12px!important;line-height:1!important}'
    + '.deepx-refresh:hover{color:#366cf6!important;border-color:#b9cbfa!important}.deepx-row{display:flex!important;justify-content:space-between!important;gap:12px!important;padding:4px 0!important;color:#5f6368!important}.deepx-btn{width:100%!important;margin-top:8px!important;padding:7px!important;border:0!important;border-radius:5px!important;background:#366cf6!important;color:#fff!important;cursor:pointer!important}.deepx-btn:disabled{opacity:.55!important;cursor:not-allowed!important}.deepx-track{height:5px!important;margin-top:9px!important;background:#e9edf2!important;border-radius:3px!important;overflow:hidden!important}.deepx-track i{display:block!important;height:100%!important;background:#366cf6!important;width:0!important;transition:width .2s!important}'
    + '.deepx-status{color:#5f6368!important;font-size:11px!important;line-height:1.5!important;margin-top:6px!important;min-height:18px!important}.deepx-error{color:#c23d3d!important}html{padding-top:40px!important;box-sizing:border-box!important}';
  const style = document.createElement('style');
  style.id = 'deepx-toolbar-style';
  style.textContent = css;
  document.head.appendChild(style);
  const internals = window.__TAURI_INTERNALS__;
  const invoke = internals && internals.invoke ? internals.invoke.bind(internals) : null;
  function getInvoke() { var i = window.__TAURI_INTERNALS__; return i && i.invoke ? i.invoke.bind(i) : null; }
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
    var ti = getInvoke();
    if (!ti) { return; }
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
  // The harness is a client-side SPA that renders into #root and can drop our
  // injected <header> / <style> (or even replace the whole document) after load.
  // Re-assert the toolbar+style so the window controls stay mounted — this is the
  // same survival guard the winbar pill used (interval + MutationObserver), since
  // on_page_load only re-fires on a full navigation, not on SPA re-renders.
  function remount() {
    try {
      if (style && !style.isConnected && document.head) { document.head.appendChild(style); }
      if (toolbar && !toolbar.isConnected && document.body) { document.body.appendChild(toolbar); }
    } catch (e) { /* best effort */ }
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
    remount();
    setInterval(remount, 600);
    if (!window.MutationObserver) { return; }
    let pending = false;
    const observer = new MutationObserver(function () {
      remount();
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
    toolbar.innerHTML = '<div class="deepx-toolbar-left"><span class="deepx-toolbar-name">DeepX Workbench</span><button class="deepx-page-reload" title="刷新">↻</button><button class="deepx-update-toggle" title="更新">更新</button></div><div class="deepx-toolbar-drag"></div><div class="deepx-toolbar-win"><button class="deepx-win deepx-win-min" title="最小化">—</button><button class="deepx-win deepx-win-max" title="最大化">O</button><button class="deepx-win deepx-win-close" title="关闭">X</button></div>';
    document.body.appendChild(toolbar);
    const drag = toolbar.querySelector('.deepx-toolbar-drag');
    drag.addEventListener('pointerdown', function (event) {
      var di = getInvoke();
      if (event.button === 0 && di) { di('window_action', { action: 'start_dragging' }).catch(function () {}); }
    });
    drag.addEventListener('dblclick', function () { win('toggle_maximize'); });
    const reloadButton = toolbar.querySelector('.deepx-page-reload');
    reloadButton.addEventListener('click', function () {
      var ri = getInvoke();
      if (reloadButton.disabled || !ri) { return; }
      reloadButton.disabled = true;
      ri('reload_harness').catch(function () {}).then(function () { reloadButton.disabled = false; });
    });
    toolbar.querySelector('.deepx-update-toggle').addEventListener('click', togglePanel);
    toolbar.querySelector('.deepx-win-min').addEventListener('click', function () { win('minimize'); });
    toolbar.querySelector('.deepx-win-max').addEventListener('click', function () { win('toggle_maximize'); });
    toolbar.querySelector('.deepx-win-close').addEventListener('click', function () { win('close'); });
    startFitting();
  }
  function win(action) { var wi = getInvoke(); if (wi) { wi('window_action', { action: action }).catch(function () {}); } }
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
  mountToolbar();
  window.__deepxToolbar = { mounted: true, remount: remount, toolbar: toolbar, hasInvoke: !!invoke, togglePanel: togglePanel };
  probe(3);
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

// 任务栏图标右键菜单（JumpList Tasks 区）：加「重启 DeepSeek Harness」。
// 点击时以 --restart-harness 再次启动本程序，由 single-instance 回调路由到
// 已运行实例执行重启，第二个实例随即退出。
#[cfg(windows)]
fn install_taskbar_restart_task() -> Result<(), String> {
    use windows::core::{w, Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::{IObjectArray, IObjectCollection};
    use windows::Win32::UI::Shell::{
        DestinationList, EnumerableObjectCollection, ICustomDestinationList, IShellLinkW,
        SetCurrentProcessExplicitAppUserModelID, ShellLink,
    };

    unsafe {
        // 必须与任务栏图标使用同一个稳定的 AppUserModelID。
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        SetCurrentProcessExplicitAppUserModelID(PCWSTR(w!("com.jeremy.deepx-workbench").as_ptr()))
            .map_err(|error| format!("AUMID: {error}"))?;

        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let exe_w: Vec<u16> = exe
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("ShellLink: {error}"))?;
        link.SetPath(PCWSTR(exe_w.as_ptr()))
            .map_err(|error| format!("SetPath: {error}"))?;
        link.SetArguments(PCWSTR(w!("--restart-harness").as_ptr()))
            .map_err(|error| format!("SetArguments: {error}"))?;
        link.SetDescription(PCWSTR(w!("重启 DeepSeek Harness").as_ptr()))
            .map_err(|error| format!("SetDescription: {error}"))?;
        link.SetIconLocation(PCWSTR(exe_w.as_ptr()), 0)
            .map_err(|error| format!("SetIconLocation: {error}"))?;

        let collection: IObjectCollection =
            CoCreateInstance(&EnumerableObjectCollection, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| format!("Collection: {error}"))?;
        collection
            .AddObject(&link)
            .map_err(|error| format!("AddObject: {error}"))?;
        let array: IObjectArray = collection
            .cast()
            .map_err(|error| format!("Array: {error}"))?;

        let destinations: ICustomDestinationList =
            CoCreateInstance(&DestinationList, None, CLSCTX_INPROC_SERVER)
                .map_err(|error| format!("DestList: {error}"))?;
        let mut max_slots: u32 = 10;
        destinations
            .BeginList::<IObjectArray>(&mut max_slots)
            .map_err(|error| format!("BeginList: {error}"))?;
        destinations
            .AddUserTasks(&array)
            .map_err(|error| format!("AddUserTasks: {error}"))?;
        destinations
            .CommitList()
            .map_err(|error| format!("CommitList: {error}"))?;
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|argument| argument == "--restart-harness") {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = commands::restart_harness(handle).await;
                });
            } else {
                activate_main(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|webview, payload| {
            let mut log = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .truncate(false)
                .open("C:\\Users\\Laptop\\AppData\\Local\\deepx-onload.log")
                .unwrap_or_else(|_| {
                    std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(false)
                        .open("C:\\Users\\Laptop\\AppData\\Local\\deepx-onload.log")
                        .expect("log")
                });
            use std::io::Write as _;
            let _ = writeln!(
                log,
                "EVT {:?}\t{} | label={}",
                payload.event(),
                payload.url(),
                webview.label()
            );
            if payload.event() == PageLoadEvent::Finished {
                let is_target = webview.label() == "main"
                    && payload.url().host_str() == Some("127.0.0.1")
                    && payload.url().port() == Some(3080);
                if is_target {
                    let result = webview.eval(TOOLBAR_SCRIPT);
                    let _ = writeln!(log, "EVAL label={} result={:?}", webview.label(), result);
                }
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
            commands::toolbar_probe,
            commands::update_deepx,
            commands::initialize_harness,
            commands::update_harness,
            commands::restart_harness,
            commands::marketplace_status,
            commands::install_marketplace,
        ])
        .setup(|app| {
            // Cap the page-load diagnostic log so long-term daily use cannot
            // grow it without bound: past ~512 KiB only the last 400 lines
            // are kept. Runs once per launch; logging itself is untouched.
            {
                const ONLOAD_LOG: &str =
                    "C:\\Users\\Laptop\\AppData\\Local\\deepx-onload.log";
                if let Ok(metadata) = std::fs::metadata(ONLOAD_LOG) {
                    if metadata.len() > 512 * 1024 {
                        if let Ok(text) = std::fs::read_to_string(ONLOAD_LOG) {
                            let lines: Vec<&str> = text.lines().collect();
                            if lines.len() > 400 {
                                let tail = lines[lines.len() - 400..].join("\n");
                                let _ = std::fs::write(ONLOAD_LOG, tail + "\n");
                            }
                        }
                    }
                }
            }
            #[cfg(windows)]
            {
                let _ = install_taskbar_restart_task();
            }
            let show = MenuItem::with_id(app, "show", "显示 DeepX", true, None::<&str>)?;
            let reload = MenuItem::with_id(app, "reload", "刷新页面", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart-harness", "重启 DeepSeek Harness", true, None::<&str>)?;
            let update = MenuItem::with_id(app, "update", "更新 DeepX", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 DeepX", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &reload, &restart, &update, &quit])?;

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
                    "restart-harness" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::restart_harness(handle).await;
                        });
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
                // Keep the main webview at 1.0 zoom so CSS px == physical px,
                // matching the 40px toolbar row that the harness page hosts.
                let _ = window.set_zoom(1.0);
            }
            // Toolbar watchdog: re-assert the injected toolbar every ~1.6s.
            // navigate()-based reloads race the on_page_load injection (the eval
            // can land in the doomed document, or a later wipe kills the timers),
            // which made the toolbar vanish after clicking refresh. The script is
            // idempotent and DOM-guarded, so while healthy this eval is a cheap
            // no-op, and after any wipe it re-mounts within one tick.
            let toolbar_watchdog = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_millis(1600));
                let handle = toolbar_watchdog.clone();
                let task = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    if let Some(webview) = task.get_webview_window("main") {
                        let _ = webview.eval(TOOLBAR_SCRIPT);
                    }
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("DeepX failed to start");
}
