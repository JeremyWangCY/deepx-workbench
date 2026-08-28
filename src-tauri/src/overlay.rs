pub(crate) fn overlay_script() -> String {
    String::from(
        r#"(() => {
if (window.__deepxOverlay?.mount) {
  window.__deepxOverlay.mount();
  return;
}
const style = document.createElement('style');
style.textContent = `
.deepx-titlebar{position:fixed;inset:0 0 auto;height:40px;z-index:2147483647;display:flex;align-items:center;background:#f8f9fa;border-bottom:1px solid #e4e7eb;color:#202124;font:13px Segoe UI,system-ui,sans-serif;user-select:none}
.deepx-titlebar-left{height:100%;display:flex;align-items:center;gap:4px;padding-left:8px}
.deepx-titlebar-name{padding:0 8px;color:#5f6368}
.deepx-titlebar-drag{height:100%;flex:1}
.deepx-page-reload,.deepx-window-button{height:100%;border:0;background:transparent;color:#68717d;cursor:pointer;font:16px/1 Segoe UI,system-ui,sans-serif}
.deepx-page-reload{width:32px;border-radius:6px;font-size:20px}
.deepx-page-reload:hover,.deepx-window-button:hover{background:#e9edf1;color:#202124}
.deepx-page-reload:disabled{opacity:.4;cursor:default}
.deepx-update-toggle{height:100%;padding:0 10px;border:0;border-radius:6px;background:transparent;color:#68717d;cursor:pointer;font:13px Segoe UI,system-ui,sans-serif}
.deepx-update-toggle:hover{background:#e9edf1;color:#202124}
.deepx-update-toggle:disabled{opacity:.55;cursor:not-allowed}
.deepx-window-controls{height:100%;display:flex}
.deepx-window-button{width:46px;font-size:17px}
.deepx-window-button.deepx-close:hover{background:#d9534f;color:#fff}
html{height:100%!important;padding-top:40px!important;box-sizing:border-box!important;overflow:hidden!important}
body,#root{height:100%!important;min-height:0!important;overflow:hidden!important}
.deepx-panel{position:fixed;top:48px;left:8px;width:min(360px,calc(100vw - 24px));padding:12px;border:1px solid #dfe3e8;border-radius:8px;background:#fff;box-shadow:0 10px 28px #0003;z-index:2147483647;font:13px Segoe UI,system-ui,sans-serif;color:#202124}
.deepx-head{align-items:center;justify-content:space-between;display:flex}
.deepx-title{font-weight:650}
.deepx-refresh{width:24px;height:24px;padding:0;border:1px solid #dfe3e8;border-radius:5px;background:#fff;color:#5f6368;cursor:pointer;font-size:12px;line-height:1}
.deepx-refresh:hover{color:#366cf6;border-color:#b9cbfa}
.deepx-row{display:flex;justify-content:space-between;gap:12px;padding:4px 0;color:#5f6368}
.deepx-btn{width:100%;margin-top:8px;padding:7px;border:0;border-radius:5px;background:#366cf6;color:#fff;cursor:pointer}
.deepx-btn:disabled{opacity:.55;cursor:not-allowed}
.deepx-track{height:5px;margin-top:9px;background:#e9edf2;border-radius:3px;overflow:hidden}
.deepx-track i{display:block;height:100%;background:#366cf6;width:0;transition:width .2s}
.deepx-status{color:#5f6368;font-size:11px;line-height:1.45;margin-top:6px;min-height:15px;white-space:pre-wrap}
.deepx-error{color:#c23d3d}
`;
document.head.appendChild(style);
let updateButton = null;
let titlebar = null;
const internals = window.__TAURI_INTERNALS__;
const invoke = internals?.invoke?.bind(internals);
let panel = null;
let busy = false;
let progressValue = 0;
let statusMessage = '';
let statusError = false;
let updateStatus = null;
let statusRequest = null;
const windowActions = { minimize: 'minimize', maximize: 'toggle_maximize', close: 'close' };
function windowCommand(action) {
  if (!invoke) return Promise.reject(new Error('窗口控制暂不可用'));
  return invoke('window_action', { action });
}
function mountTitlebar() {
  if (titlebar?.isConnected) return;
  panel?.remove();
  panel = null;
  titlebar = document.createElement('header');
  titlebar.className = 'deepx-titlebar';
  titlebar.innerHTML = `
<div class="deepx-titlebar-left"><button class="deepx-page-reload" title="刷新页面" aria-label="刷新页面">↻</button><span class="deepx-titlebar-name">DeepX</span><button class="deepx-update-toggle" title="更新" aria-label="更新">更新</button></div>
<div class="deepx-titlebar-drag" data-tauri-drag-region></div>
<div class="deepx-window-controls"><button class="deepx-window-button" data-action="minimize" title="最小化" aria-label="最小化">−</button><button class="deepx-window-button" data-action="maximize" title="最大化" aria-label="最大化">□</button><button class="deepx-window-button deepx-close" data-action="close" title="关闭" aria-label="关闭">×</button></div>`;
  document.body.appendChild(titlebar);
  titlebar.querySelector('.deepx-titlebar-drag').onpointerdown = event => {
    if (event.button === 0) windowCommand('start_dragging').catch(error => setStatus(String(error), true));
  };
  const reloadButton = titlebar.querySelector('.deepx-page-reload');
  reloadButton.onclick = async event => {
    event.stopPropagation();
    if (reloadButton.disabled || !invoke) return;
    reloadButton.disabled = true;
    try { await invoke('reload_harness'); }
    catch (error) { reloadButton.disabled = false; setStatus(String(error), true); }
  };
  updateButton = titlebar.querySelector('.deepx-update-toggle');
  updateButton.onclick = event => {
    event.stopPropagation();
    if (panel) { panel.remove(); panel = null; return; }
    panel = document.createElement('div');
    panel.className = 'deepx-panel';
    document.body.appendChild(panel);
    drawPanel();
  };
  titlebar.querySelectorAll('[data-action]').forEach(button => {
    button.onclick = async event => {
      event.stopPropagation();
      try { await windowCommand(windowActions[button.dataset.action]); }
      catch (error) { setStatus(String(error), true); }
    };
  });
}
function setBusy(next, message = '') {
  busy = next;
  panel?.querySelectorAll('.deepx-btn').forEach(button => button.disabled = busy);
  const refreshButton = panel?.querySelector('.deepx-refresh');
  if (refreshButton) refreshButton.disabled = busy;
  setStatus(message);
}
function setStatus(message, isError = false) {
  statusMessage = message;
  statusError = isError;
  const output = panel?.querySelector('.deepx-status');
  if (!output) return;
  output.textContent = statusMessage;
  output.classList.toggle('deepx-error', statusError);
}
function setProgress(value) {
  progressValue = Math.max(0, Math.min(100, Number(value) || 0));
  const indicator = panel?.querySelector('.deepx-track i');
  if (indicator) indicator.style.width = `${progressValue}%`;
}
function versionText(status) {
  if (!status) return '未检查';
  const current = status.current || '未安装';
  const latest = status.latest || '未知';
  return `当前 ${current} · 最新 ${latest}`;
}
function actionLabel(status, target) {
  if (!status || !status.current) return `安装 ${target}`;
  return status.update_available ? `更新 ${target}` : '';
}
function applyStatus(status) {
  updateStatus = status;
  if (!panel) return;
  panel.querySelector('.deepx-app-version').textContent = versionText(status.deepx);
  panel.querySelector('.deepx-version').textContent = versionText(status.harness);
  panel.querySelector('.deepx-market-version').textContent = versionText(status.marketplace);
  renderActions();
}
function refreshStatus() {
  if (!invoke) return Promise.resolve();
  if (statusRequest) return statusRequest;
  statusRequest = invoke('update_status').then(applyStatus).finally(() => { statusRequest = null; });
  return statusRequest;
}
function renderActions() {
  const actions = panel?.querySelector('.deepx-actions');
  if (!actions) return;
  actions.innerHTML = '';
  const definitions = [
    ['deepx', 'deepx-app-update', 'update_deepx', 'DeepX'],
    ['harness', 'deepx-update', 'update_harness', 'Harness'],
    ['marketplace', 'deepx-market-update', 'install_marketplace', '插件市场'],
  ];
  definitions.forEach(([key, className, command, target]) => {
    const label = actionLabel(updateStatus?.[key], target);
    if (!label) return;
    const button = document.createElement('button');
    button.className = `deepx-btn ${className}`;
    button.textContent = label;
    button.disabled = busy;
    button.onclick = () => runAction(command, key);
    actions.appendChild(button);
  });
}
async function runAction(command, key) {
  if (busy || !invoke) return;
  try {
    setBusy(true, key === 'deepx' ? '正在下载 DeepX 更新...' : key === 'harness' ? '正在更新 Harness...' : '正在准备插件市场...');
    await invoke(command);
    setBusy(false, key === 'deepx' ? 'DeepX 更新安装器已启动' : key === 'harness' ? 'Harness 已更新' : '插件市场已更新');
    if (updateStatus?.[key]) {
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
  if (!panel) return;
  panel.innerHTML = `
<div class="deepx-head"><span class="deepx-title">DeepX</span><button class="deepx-refresh" title="刷新状态">↻</button></div>
<div class="deepx-row"><span>DeepX</span><span class="deepx-app-version">未检查</span></div>
<div class="deepx-row"><span>Harness</span><span class="deepx-version">未检查</span></div>
<div class="deepx-row"><span>插件市场</span><span class="deepx-market-version">未检查</span></div>
<div class="deepx-actions"></div>
<div class="deepx-track"><i></i></div>
<div class="deepx-status"></div>`;
  setProgress(progressValue);
  setStatus(statusMessage, statusError);
  applyStatus(updateStatus || { deepx: null, harness: null, marketplace: null });
  const refreshButton = panel.querySelector('.deepx-refresh');
  refreshButton.onclick = async () => {
    if (busy || refreshButton.disabled) return;
    refreshButton.disabled = true;
    setStatus('正在刷新状态...');
    try {
      await refreshStatus();
      setStatus('状态已刷新');
    } catch (error) {
      setStatus(String(error), true);
    } finally {
      refreshButton.disabled = false;
    }
  };
}
function mount() {
  if (!document.body) return;
  mountTitlebar();
}
if (internals?.transformCallback && internals?.invoke) {
  internals.invoke('plugin:event|listen', {
    event: 'runtime-progress',
    handler: internals.transformCallback(payload => {
      setProgress(payload?.percentage || 0);
      setStatus(payload?.detail || '');
    }),
  }).catch(() => {});
}
window.__deepxOverlay = { mount };
mount();
void refreshStatus().catch(() => {});
new MutationObserver(mount).observe(document.documentElement, { childList: true, subtree: true });
})();
"#,
    )
}
