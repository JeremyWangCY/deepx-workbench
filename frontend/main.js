import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

const root = document.querySelector("#app");
const state = { ready: false, progress: 0, detail: "正在准备...", error: "", reloading: false };

function windowAction(action) {
  return invoke("window_action", { action });
}

function bindWindowControls(titlebar) {
  titlebar.querySelector(".titlebar-drag").addEventListener("pointerdown", (event) => {
    if (event.button === 0) void windowAction("start_dragging");
  });
  titlebar.querySelectorAll("[data-window-action]").forEach((button) => {
    button.addEventListener("click", async (event) => {
      event.stopPropagation();
      try {
        const action = button.dataset.windowAction;
        if (action === "minimize") await windowAction("minimize");
        if (action === "maximize") await windowAction("toggle_maximize");
        if (action === "close") await windowAction("close");
      } catch (error) {
        state.error = String(error);
        render();
      }
    });
  });
}

function mountTitlebar() {
  const titlebar = document.createElement("header");
  titlebar.className = "titlebar";
  titlebar.innerHTML = `
    <div class="titlebar-left">
      <button class="titlebar-button page-reload" type="button" title="刷新页面" aria-label="刷新页面">↻</button>
      <span class="titlebar-name">DeepX</span>
    </div>
    <div class="titlebar-drag" data-tauri-drag-region></div>
    <div class="window-controls">
      <button class="window-button" data-window-action="minimize" type="button" title="最小化" aria-label="最小化">−</button>
      <button class="window-button" data-window-action="maximize" type="button" title="最大化" aria-label="最大化">□</button>
      <button class="window-button close" data-window-action="close" type="button" title="关闭" aria-label="关闭">×</button>
    </div>`;
  root.appendChild(titlebar);
  bindWindowControls(titlebar);
  titlebar.querySelector(".page-reload").addEventListener("click", async () => {
    if (!state.ready || state.reloading) return;
    state.reloading = true;
    titlebar.querySelector(".page-reload").disabled = true;
    try {
      await invoke("reload_harness");
    } catch (error) {
      state.reloading = false;
      titlebar.querySelector(".page-reload").disabled = false;
      state.error = String(error);
      render();
    }
  });
  return titlebar;
}

const titlebar = mountTitlebar();

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
  titlebar.querySelector(".page-reload").disabled = !state.ready || state.reloading;
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
