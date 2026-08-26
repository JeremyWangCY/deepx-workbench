pub(crate) fn overlay_script() -> String {
    String::from(
        r#"(() => {
if (window.__deepxOverlay?.mount) {
  window.__deepxOverlay.mount();
  return;
}
const style = document.createElement('style');
style.textContent = `
.deepx-box{position:fixed;left:0;bottom:58px;width:56px;padding:0 8px;box-sizing:border-box;z-index:2147483647;font:13px Segoe UI,system-ui,sans-serif;color:#202124}
.deepx-toggle{display:flex;align-items:center;justify-content:center;gap:8px;width:100%;height:40px;padding:0;border:0;border-radius:8px;background:transparent;color:#5f6368;box-shadow:none;cursor:pointer;font:inherit;text-align:left}
.deepx-toggle:hover{background:#eef0f2;color:#202124}
.deepx-toggle:disabled{opacity:.55;cursor:not-allowed}
.deepx-toggle-icon{font-size:17px;line-height:1}
.deepx-toggle-label{white-space:nowrap}
.deepx-box.compact .deepx-toggle-label{display:none}
.deepx-box.expanded{width:100%;max-width:360px}
.deepx-box.expanded .deepx-toggle{justify-content:flex-start;padding:0 12px}
.deepx-panel{width:min(360px,calc(100vw - 24px));margin-bottom:8px;padding:12px;border:1px solid #dfe3e8;border-radius:8px;background:#fff;box-shadow:0 10px 28px #0003}
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
let box = null;
const internals = window.__TAURI_INTERNALS__;
const invoke = internals?.invoke?.bind(internals);
let panel = null;
let busy = false;
let progressValue = 0;
let statusMessage = '';
let statusError = false;
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
  if (!status) return '不可用';
  const current = status.current || '未安装';
  const latest = status.latest || '未知';
  return `当前 ${current} · 最新 ${latest}`;
}
function refresh() {
  return invoke?.('update_status').then(status => {
    panel.querySelector('.deepx-app-version').textContent = versionText(status.deepx);
    panel.querySelector('.deepx-version').textContent = versionText(status.harness);
    panel.querySelector('.deepx-market-version').textContent = versionText(status.marketplace);
  }).catch(() => {
    panel.querySelectorAll('.deepx-version, .deepx-app-version, .deepx-market-version').forEach(output => output.textContent = '不可用');
  });
}
function drawPanel() {
  if (!panel) return;
  panel.innerHTML = `
<div class="deepx-head"><span class="deepx-title">DeepX</span><button class="deepx-refresh" title="刷新状态">↻</button></div>
<div class="deepx-row"><span>DeepX</span><span class="deepx-app-version">检查中...</span></div>
<div class="deepx-row"><span>Harness</span><span class="deepx-version">检查中...</span></div>
<div class="deepx-row"><span>插件市场</span><span class="deepx-market-version">检查中...</span></div>
<button class="deepx-btn deepx-app-update">更新 DeepX</button>
<button class="deepx-btn deepx-update">更新 Harness</button>
<button class="deepx-btn deepx-market-update">安装 / 更新插件市场</button>
<div class="deepx-track"><i></i></div>
<div class="deepx-status"></div>`;
  panel.querySelectorAll('.deepx-btn').forEach(button => button.disabled = busy);
  setProgress(progressValue);
  setStatus(statusMessage, statusError);
  panel.querySelector('.deepx-app-update').onclick = async () => {
    if (busy || !invoke) return;
    try { setBusy(true, '正在下载并启动 DeepX 更新...'); await invoke('update_deepx'); setBusy(false, 'DeepX 更新安装器已启动'); }
    catch (error) { setBusy(false, String(error), true); }
  };
  panel.querySelector('.deepx-update').onclick = async () => {
    if (busy || !invoke) return;
    try { setBusy(true, '开始更新...'); await invoke('update_harness'); setBusy(false, 'Harness 已更新'); refresh(); }
    catch (error) { setBusy(false, String(error), true); setProgress(0); }
  };
  panel.querySelector('.deepx-market-update').onclick = async () => {
    if (busy || !invoke) return;
    try { setBusy(true, '准备插件市场...'); await invoke('install_marketplace'); setBusy(false, '插件市场已就绪'); refresh(); }
    catch (error) { setBusy(false, String(error), true); setProgress(0); }
  };
  const refreshButton = panel.querySelector('.deepx-refresh');
  refreshButton.onclick = () => {
    if (busy || refreshButton.disabled) return;
    refreshButton.disabled = true;
    setStatus('正在刷新状态...');
    refresh().then(() => setStatus('状态已刷新')).finally(() => refreshButton.disabled = false);
  };
  refresh();
}
function syncSidebar() {
  if (!box) return;
  const candidates = [...document.body.querySelectorAll('*')].filter(element => {
    if (element.closest('.deepx-box')) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.left <= 4 && rect.top <= 4 && rect.height >= innerHeight * 0.7 &&
      rect.width >= 48 && rect.width <= Math.min(innerWidth * 0.8, 420) &&
      ['fixed', 'sticky'].includes(style.position);
  });
  const width = Math.round(candidates.reduce((largest, element) =>
    Math.max(largest, element.getBoundingClientRect().width), 56));
  box.classList.toggle('expanded', width > 96);
  box.classList.toggle('compact', width <= 96);
  box.style.width = Math.min(width, 360) + 'px';
}

function mount() {
  if (!document.body || document.querySelector('.deepx-box')) return;
  panel = null;
  box = document.createElement('div');
  box.className = 'deepx-box';
  box.innerHTML = '<button class="deepx-toggle" title="DeepX、Harness 与插件市场更新"><span class="deepx-toggle-icon">↻</span><span class="deepx-toggle-label">更新</span></button>';
  document.body.appendChild(box);
  syncSidebar();
  box.querySelector('.deepx-toggle').onclick = () => {
    if (panel) { panel.remove(); panel = null; return; }
    panel = document.createElement('div');
    panel.className = 'deepx-panel';
    box.prepend(panel);
    drawPanel();
  };
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
new MutationObserver(() => { mount(); syncSidebar(); }).observe(document.documentElement, { childList: true, subtree: true });
window.addEventListener('resize', syncSidebar);
})();
"#,
    )
}
