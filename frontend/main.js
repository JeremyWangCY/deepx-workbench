import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

const root = document.querySelector("#app");
const state = { ready: false, progress: 0, detail: "正在准备...", error: "" };

function render() {
  const page = root.querySelector("main");
  if (page) page.remove();
  const main = document.createElement("main");
  if (state.ready) {
    main.className = "ready";
    main.innerHTML = '<strong>DeepX</strong><span>正在打开...</span>';
  } else {
    main.innerHTML = '<section class="panel"><strong class="brand">DeepX</strong><div class="detail"></div><div class="track"><i></i></div><pre class="error"></pre></section>';
    main.querySelector(".detail").textContent = state.detail;
    main.querySelector(".track i").style.width = `${state.progress}%`;
    main.querySelector(".error").textContent = state.error;
  }
  root.appendChild(main);
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