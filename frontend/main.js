import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

const root = document.querySelector("#app");
const state = { ready: false, progress: 0, detail: "检查本机 Harness...", error: "" };
function render() {
  root.innerHTML = state.ready
    ? '<main class="ready"><strong>DeepX</strong><span>正在打开 DeepSeek Harness...</span></main>'
    : '<main><section class="panel"><div class="eyebrow">DeepX Workbench</div><h1>DeepSeek Harness</h1><p>轻量桌面壳。首次运行安装一次，之后直接进入 Harness。</p><div class="detail">' + state.detail + '</div><div class="track"><i style="width:' + state.progress + '%"></i></div>' + (state.error ? '<pre class="error">' + state.error + '</pre>' : '') + '</section></main>';
}
render();
void listen("runtime-progress", (event) => {
  state.progress = Number(event.payload?.percentage || 0);
  state.detail = event.payload?.detail || state.detail;
  render();
});
async function boot() {
  try {
    const status = await invoke("runtime_status");
    if (!status.ready) {
      await invoke("update_harness");
      return;
    }
    await invoke("launch_harness");
    state.ready = true;
    render();
    await invoke("show_harness");
  } catch (error) {
    state.error = String(error);
    render();
  }
}
void boot();
