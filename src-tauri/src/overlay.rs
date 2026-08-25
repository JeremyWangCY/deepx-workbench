pub(crate) fn overlay_script() -> String {
    String::from(
        r#"(() => {
if (window.__deepxOverlay?.mount) {
  window.__deepxOverlay.mount();
  return;
}
const style = document.createElement('style');
style.textContent = `
.deepx-box{position:fixed;left:12px;bottom:58px;z-index:2147483647;font:13px Segoe UI,system-ui,sans-serif;color:#202124}
.deepx-toggle{height:34px;padding:0 11px;border:1px solid #dfe3e8;border-radius:7px;background:#fff;box-shadow:0 2px 10px #0002;cursor:pointer}
.deepx-panel{width:min(320px,calc(100vw - 24px));margin-bottom:8px;padding:12px;border:1px solid #dfe3e8;border-radius:8px;background:#fff;box-shadow:0 10px 28px #0003}
.deepx-head{align-items:center;justify-content:space-between;display:flex}
.deepx-title{font-weight:650}
.deepx-refresh{width:24px;height:24px;padding:0;border:1px solid #dfe3e8;border-radius:5px;background:#fff;color:#5f6368;cursor:pointer;font-size:12px;line-height:1}
.deepx-refresh:hover{color:#366cf6;border-color:#b9cbfa}
.deepx-row{display:flex;justify-content:space-between;gap:12px;padding:4px 0;color:#5f6368}
.deepx-seg{display:inline-flex;border:1px solid #dfe3e8;border-radius:5px;overflow:hidden}
.deepx-channel{padding:2px 7px;border:0;background:#fff;color:#5f6368;cursor:pointer;font-size:11px}
.deepx-channel.active{background:#366cf6;color:#fff}
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
function setBusy(next, message = '') {
  busy = next;
  panel?.querySelectorAll('.deepx-btn').forEach(button => button.disabled = busy);
  setStatus(message);
}
function setStatus(message, isError = false) {
  const output = panel?.querySelector('.deepx-status');
  if (!output) return;
  output.textContent = message;
  output.classList.toggle('deepx-error', isError);
}
function setProgress(value) {
  const indicator = panel?.querySelector('.deepx-track i');
  if (indicator) indicator.style.width = `${Math.max(0, Math.min(100, Number(value) || 0))}%`;
}
function refresh() {
  return Promise.all([
    invoke?.('update_status').then(status => {
      const installed = status.installed_version || '未安装';
      const latest = status.latest_version || '未知';
      panel.querySelector('.deepx-version').textContent = status.update_available ? `${installed} → ${latest}` : `${installed} 最新`;
    }).catch(() => panel.querySelector('.deepx-version').textContent = '不可用'),
    invoke?.('marketplace_status').then(status => {
      panel.querySelector('.deepx-market').textContent = status.installed ? '已安装' : '尚未安装';
    }).catch(() => panel.querySelector('.deepx-market').textContent = '不可用'),
  ]);
}
function setActiveChannel(channel) {
  panel?.querySelectorAll('.deepx-channel').forEach(button => {
    button.classList.toggle('active', button.dataset.channel === channel);
  });
}
function drawPanel() {
  if (!panel) return;
  panel.innerHTML = `
<div class="deepx-head"><span class="deepx-title">DeepX</span><button class="deepx-refresh" title="刷新状态">↻</button></div>
<div class="deepx-row"><span>Harness</span><span class="deepx-version">检查中...</span></div>
<div class="deepx-row"><span>更新通道</span><span class="deepx-seg"><button class="deepx-channel" data-channel="latest">latest</button><button class="deepx-channel" data-channel="next">next</button></span></div>
<div class="deepx-row"><span>插件市场</span><span class="deepx-market">检查中...</span></div>
<button class="deepx-btn deepx-app-update">更新 DeepX</button>
<button class="deepx-btn deepx-update">更新 Harness</button>
<button class="deepx-btn deepx-market">安装 / 更新插件市场</button>
<div class="deepx-track"><i></i></div>
<div class="deepx-status"></div>`;
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
  panel.querySelector('.deepx-btn.deepx-market').onclick = async () => {
    if (busy || !invoke) return;
    try { setBusy(true, '准备插件市场...'); await invoke('install_marketplace'); setBusy(false, '插件市场已就绪'); refresh(); }
    catch (error) { setBusy(false, String(error), true); setProgress(0); }
  };
  panel.querySelectorAll('.deepx-channel').forEach(button => {
    button.onclick = async () => {
      if (busy || !invoke) return;
      const channel = button.dataset.channel;
      try { await invoke('select_update_channel', { selection: { channel } }); setActiveChannel(channel); refresh(); }
      catch (error) { setStatus(String(error), true); }
    };
  });
  const refreshButton = panel.querySelector('.deepx-refresh');
  refreshButton.onclick = () => {
    if (busy || refreshButton.disabled) return;
    refreshButton.disabled = true;
    setProgress(0);
    setStatus('正在刷新状态...');
    refresh().then(() => setStatus('状态已刷新')).finally(() => refreshButton.disabled = false);
  };
  invoke?.('get_update_channel').then(status => setActiveChannel(status.channel)).catch(() => {});
  refresh();
}
function mount() {
  if (!document.body || document.querySelector('.deepx-box')) return;
  panel = null;
  box = document.createElement('div');
  box.className = 'deepx-box';
  box.innerHTML = '<button class="deepx-toggle" title="DeepX、Harness 与插件市场更新">↻ 更新</button>';
  document.body.appendChild(box);
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
new MutationObserver(mount).observe(document.documentElement, { childList: true, subtree: true });
})();
"#,
    )
}
