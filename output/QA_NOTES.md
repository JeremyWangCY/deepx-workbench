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
