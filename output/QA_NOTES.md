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

## 2026-08-25 follow-up verification

- Verified both Start Menu and Desktop shortcuts point to:
  `%LOCALAPPDATA%\DeepX Workbench\deepx-workbench.exe`, with working directory
  `%LOCALAPPDATA%\DeepX Workbench`. Launching through the Start Menu shortcut
  created a visible main window and booted Harness on `127.0.0.1:3080` (HTTP
  200).
- Reopened Settings over CDP and confirmed the real marketplace entry appears
  as a “插件市场” settings tab. Opening it rendered `dsh-market v1.20.2`, its
  category filters, plugin listings with Install buttons, pagination, and the
  “升级市场” action.
- Added the persisted latest/next update channel selector. CDP verification:
  both IPC reads/writes succeed, the segmented control tracks the active
  channel, update-channel.json persists across calls, and update_status resolves
  both dist-tags (0.1.1-rc.2 at verification time). The test machine was
  restored to latest.

- Split status refresh from updating: the panel header now has a dedicated
  compact refresh button, while the main action is labeled “更新 Harness”.
  CDP verified a 24x24 refresh control, absence of the combined label, and a
  successful refresh ending in “状态已刷新”.

## 2026-08-25 shell ownership fix

- Replaced the positional launcher form with the documented
  \`dsh --profile web\` invocation.
- Added a persistent profile patch that forces \`openBrowser: false\`, in
  addition to \`--no-open\`; verified the generated Node command includes both
  safeguards and writes \`deepx-no-open.yml\` under DSH_HOME.
- Added single-instance activation, close-to-tray behavior, and a tray menu.
  A second shortcut exited while the first instance remained active. After
  closing the first window to the tray, another shortcut restored/focused it.

### Shared DSH_HOME follow-up

- Moved DeepX plugin/profile handling to the standard \`~/.dsh\` home.
- Added a one-time migration that merges community dependencies and bundle
  entries from the former private profile into \`~/.dsh/profiles/web\`, then
  installs missing packages through the official CLI.
- Verified migration produced \`dshmarket ^1.26.0\` in the shared manifest and
  the market API reports it present/live.
- Confirmed dshmarket v1.26 deliberately excludes its own package name from
  the Installed tab; with no other community packages that tab therefore shows
  the empty-state message even though the market component is installed.
- Removed the former private \`profiles\` directory after successful
  migration on the test machine.

### Release audit

- Tightened marketplace detection so a dependency must also have its installed
  package manifest present.
- Migration now reports whether it changed the active home, restarts an old
  private-home service after migration, tolerates missing manifest sections,
  and stores its marker outside the shared Harness profile.
- Close-to-tray handling is scoped to the main window.
- Final packaged build passed typecheck, rustfmt, clippy, release build, and a
  shortcut launch with Harness HTTP 200.
