# DeepX Workbench QA Notes

QA date: 2026-08-24
Platform: Windows, x64, Tauri + Vite build.

## What was verified

### First install (one-time)

- A clean-room install of the official runtime `@deepseek-ai/dsh@latest` was run
  against an empty prefix.
- Result: success. `npm install` added **511 packages**, ~212 MB on disk.
- Time: **about 18 minutes** on this machine. This is the large dependency tree
  of the official Harness package, and it only happens once per machine.
- The npm package reported the `dsh` bin present after install
  (`HAS_BIN=True`).

Note: no dependency installation runs on later launches. The app only installs
when `runtime_status.ready` is false (no Node runtime + no Harness in the app
data folder).

### Normal launch (no reinstall)

- A pre-seeded runtime was placed at the Tauri app-data path.
- Launching the packaged `deepx-workbench.exe` went straight to the Harness
  boot flow. The Harness HTTP service came up on `127.0.0.1:3080` and returned
  200 after **~9 seconds**. No dependency install was triggered.
- Screenshot: `qa-direct-open.png`.

### Update flow

- The update control is injected live into the Harness page in the lower-left
  area (above the settings gear), not inside the Harness settings page.
- It lists the installed vs latest Harness version and runs the update path:
  stop service -> npm install latest -> relaunch -> navigate back.
- Progress events are surfaced on the same control (percentage + status text).
- Screenshots from earlier passes: `qa-update-15s.png`, `qa-after-click.png`.

### Marketplace

- The same control offers a one-click "install / update plugin marketplace"
  button.
- Flow: ensure runtime -> `dsh plugin --profile web add dshmarket` -> verify the
  dependency is recorded in the web profile -> report progress.
- Status is shown inline ("已安装 / 尚未安装").

## Residual notes

- `qa-update-progress.png` from an earlier pass is a blank capture; it was
  removed from the tracked set.
- The remaining screenshots are illustrative captures taken during manual QA,
  not a full automated suite. Re-run a packaged build before cutting a release.

## 2026-08-24 interactive verification (DevTools / CDP)

Verified by driving the real packaged build over WebView2 DevTools from a local
Node CDP helper (no screenshots needed; DOM state read directly):

- IPC into the remote Harness page works after the ACL work: `runtime_status`,
  `update_status`, `marketplace_status` all return real data from the overlay.
- Bug found: `update_status` returns snake_case keys (`installed_version`,
  `latest_version`, `update_available`), but the overlay read camelCase
  (`installedVersion`...), so the panel showed the version as "未安装". Fixed in
  `overlay.rs` to read the snake_case keys. Panel now shows `0.1.1-rc.2 最新`.
- Bug found: the marketplace button selector `.deepx-market` matched the status
  span first (document order), so the click handler attached to the `<span>` and
  the real button had no `onclick`. Changed to `.deepx-btn.deepx-market`. The
  button now runs `install_marketplace` ("准备插件市场..." -> "插件市场已就绪").
- Clicked the update button end to end: stops the harness service, reinstalls
  the runtime, relaunches `web --no-open --port 3080`, and navigates back. The
  service returned 200 after the cycle with a fresh process.
- Marketplace one-click install verified: adds `dshmarket ^1.20.2` to the web
  profile and pulls its dependency tree. After install the panel reports
  "已安装" and `marketplace_status.installed` is true.
- Launches do not open a browser (`--no-open`); a launch with an already-seeded
  runtime goes straight to the Harness without reinstalling dependencies.
- Note: a plain `cargo build --release` embeds the dev URL, so the tray/webview
  pointed at `localhost:1420`. The correct build path is `pnpm tauri build
  --no-bundle` (sets prod config and embeds `dist/`).
