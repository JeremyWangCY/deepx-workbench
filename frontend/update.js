import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const $ = (id) => document.getElementById(id);
const bar = $("bar");
const sub = $("sub");
const version = $("version");
const statusEl = $("status");
const bytesEl = $("bytes");
const speedEl = $("speed");
const startBtn = $("start");
const hint = $("hint");

function fmtBytes(n) {
  if (n == null) return "--";
  const mb = n / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(1)} MB`;
}

function setStatus(text, cls) {
  statusEl.textContent = text;
  statusEl.className = cls || "";
}

async function main() {
  let st;
  try {
    st = await invoke("update_status");
  } catch (e) {
    sub.textContent = "读取更新状态失败";
    hint.textContent = String(e);
    return;
  }
  const d = st.deepx;
  version.textContent = `当前 v${d.current ?? "--"} · 最新 v${d.latest ?? "--"}`;
  if (d.update_available) {
    setStatus("可更新", "ok");
    startBtn.disabled = false;
    sub.textContent = `有新版本 v${d.latest} 可以下载`;
  } else {
    setStatus("已是最新", "latest");
    sub.textContent = "DeepX 已是最新版本";
  }
}

startBtn.addEventListener("click", async () => {
  startBtn.disabled = true;
  startBtn.textContent = "下载中…";
  bar.style.width = "0%";
  bytesEl.textContent = "-- / --";
  speedEl.textContent = "-- MB/s";
  hint.textContent = "";
  sub.textContent = "正在准备下载…";
  try {
    await invoke("update_deepx");
  } catch (e) {
    sub.textContent = "更新失败";
    hint.textContent = String(e);
    startBtn.textContent = "重试";
    startBtn.disabled = false;
    bar.style.width = "0%";
  }
});

await listen("deepx-update-progress", (event) => {
  const p = event.payload;
  if (!p || typeof p.percent !== "number") return;
  bar.style.width = `${Math.max(0, Math.min(p.percent, 100))}%`;
  bytesEl.textContent = `${fmtBytes(p.downloaded)} / ${fmtBytes(p.total)}`;
  const mbps = p.speed > 0 ? (p.speed / (1024 * 1024)).toFixed(1) : "--";
  speedEl.textContent = `${mbps} MB/s`;
  if (p.detail) sub.textContent = p.detail;
  if (p.percent >= 100) {
    sub.textContent = "下载完成，正在启动安装器…";
    hint.textContent = `共下载 ${fmtBytes(p.downloaded)}`;
  }
});

await listen("runtime-progress", (event) => {
  const payload = event.payload;
  if (payload && typeof payload.detail === "string" && payload.detail) {
    const d = String(payload.detail);
    if (d === "DeepX 已是最新版本") {
      setStatus("已是最新", "latest");
      startBtn.disabled = true;
    }
    if (!d.startsWith("正在下载")) hint.textContent = d;
  }
});

main();
