import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

const root = document.querySelector("#app");
const state = { ready: false, progress: 0, detail: "正在准备...", error: "" };
function render() {
  root.innerHTML = state.ready
    ? '<main class="ready"><strong>DeepX</strong><span>正在打开...</span></main>'
    : '<main><section class="panel"><strong class="brand">DeepX</strong><div class="detail">' + state.detail + '</div><div class="track"><i style="width:' + state.progress + '%"></i></div>' + (state.error ? '<pre class="error">' + state.error + '</pre>' : '') + '</section></main>';
}
render();
void listen("runtime-progress", (event) => {
  state.progress = Number(event.payload?.percentage || 0);
  state.detail = event.payload?.detail || state.detail;
  render();
});
async function boot() {
  try {
    const [status, marketplace] = await Promise.all([
      invoke("runtime_status"),
      invoke("marketplace_status"),
    ]);
    if (!status.ready || !marketplace.installed) {
      await invoke("initialize_harness");
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
