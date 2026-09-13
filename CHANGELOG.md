# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.73] - 2026-09-13

### Added

- Added an explicit Harness lifecycle model (`stopped`, `starting`, `healthy`, `recovering`, `updating`, `rolling_back`, `failed`) and surfaced those states in the DeepX settings panel, including live refresh while automatic recovery or rollback is in progress.
- Added a one-click **Export redacted diagnostics** action. The generated ZIP contains only a compact runtime summary plus recent Harness startup/supervisor logs; authentication query tokens and the user home path are redacted, while credentials and Harness session files are not included.
- First-run runtime downloads now show transferred MB, total size when known, and current MB/s, with clearer GitHub Release connectivity errors and cleanup of incomplete archives.

### Changed

- Cached the already-discovered Harness loopback endpoint during healthy operation so routine status checks and the watchdog avoid repeatedly spawning PowerShell/CIM listener-discovery commands. The cache is cleared on stop, process exit, or health failure and the existing exact-process discovery path remains the fallback.
- Harness supervisor logs now rotate at 2 MiB instead of growing without bound.
- Token query detection and route sanitization are now case-insensitive.

### Fixed

- Prevented intentional restart/update child exits from briefly overwriting the active maintenance lifecycle with `stopped`.
- Prevented a single settings-panel health probe from downgrading the lifecycle state; recovery decisions remain owned by the supervisor/watchdog.

## [0.1.72] - 2026-09-12

### Fixed

- Hid the Windows helper window used by the single-instance plugin so launching DeepX again activates the existing instance without leaving a visible helper window behind.
- Vendored the patched single-instance plugin used by the Windows build so the helper-window behavior is deterministic in packaged releases.

## [0.1.71] - 2026-09-11

### Changed

- Simplified the DeepX settings panel around actionable information: connection status and the current local Harness endpoint, file shortcuts, maintenance actions, and a compact DeepX/Harness version summary. Static implementation details such as WebView2/Tauri, F12, and close-to-tray behavior no longer occupy the settings surface.
- Settings now read DeepX and Harness versions from local runtime status instead of triggering a network update check just to render the panel.
- Settings actions keep short, stable button labels while progress and errors are reported in a dedicated status line; runtime status starts at “checking” instead of guessing from the current page.
- The startup shell now keeps a single “正在准备...” state until Harness takes over the WebView, removing the redundant “DeepX 正在打开...” transition.

## [0.1.70] - 2026-09-11

### Added

- Added a persistent Harness supervisor: DeepX now retains process ownership, watches unexpected exits, detects repeated health failures, and automatically recovers the Harness with bounded exponential backoff while keeping the desktop shell alive.
- Added transactional Harness updates: updates are prepared in an isolated `runtime.staging`, validated before activation, atomically swapped with the active runtime, and automatically rolled back to `runtime.previous` when the new Harness cannot start.
- Added a separate `harness-supervisor.log` with spawn, health, exit, recovery, and rollback events; startup logs now append and rotate instead of being truncated on every launch.
- Added DeepX-owned route memory so reconnecting after a Harness restart can return to the last WebView route without modifying Harness session storage; authentication query parameters are never persisted.
- Added CI regression tests for Harness token redaction and route sanitization.

### Changed

- Harness process management is now ownership-aware and endpoint-driven. DeepX identifies its own Web Harness from the runtime entrypoint, DeepX patch, profile, and launch mode, then discovers the process's actual loopback listener instead of owning a fixed port.
- DeepX now launches new Harness processes on an automatically selected free port and reconnects to existing older fixed-port instances during upgrades, eliminating port-3080 collisions without changing Harness itself.
- Runtime staging excludes disposable pnpm store data and symlinks so update candidates cannot accidentally dereference links into profiles, temporary projects, or other user paths.
- Runtime validation now checks required binaries, marker/package version agreement, and aligned DeepSeek Harness package versions before a staged runtime can become active.

### Fixed

- Removed the invasive titlebar layout workaround that combined `html { padding-top: 40px }`, overlay-specific overrides, and periodic mutation of arbitrary fixed-position Harness DOM nodes. Harness pages are now inset once at the `#root` boundary while DeepX only maintains DeepX-owned toolbar nodes.
- Fixed false-positive Harness health/ownership cases by binding health checks to the listener owned by the exact DeepX-launched process, including Windows command-line path normalization and stale/unresponsive owned processes being restarted safely instead of spawning a duplicate listener.
- Prevented Harness authentication tokens from being persisted in DeepX startup or page-load diagnostics: output is parsed in memory, token values are redacted before disk writes, and legacy startup logs are sanitized on the next launch.
- Restricted authentication-cookie and route-memory commands to the WebView currently attached to DeepX's actual Harness endpoint, while keeping settings/status visibility available to the local DeepX shell.
- Removed development-machine-specific diagnostic paths; WebView diagnostics are now opt-in, stored under DeepX's own app log directory, size-capped, and URL-redacted.
- Prevented intentional Harness restart/update operations from racing the recovery watchdog or creating duplicate recovery loops.
- Added interrupted-update boot recovery: if a transactional swap is interrupted after the old runtime was moved aside, DeepX restores a validated `runtime.previous` before falling back to reinstalling the bundled runtime.

## [0.1.69] - 2026-09-07

### Added

- Added developer tools & quick access directories to the DeepX settings panel: users can now open the configuration directory (~/.dsh), plugins directory (Web Profile), skills directory (.agents/skills), inspect startup logs (harness-startup.log), and repair plugin metadata directly with one click.
- Added F12 keyboard shortcut to toggle WebView2 developer tools (window.open_devtools()).
- Added cross-Harness skill compatibility (ensure_cross_harness_compatibility / migrate_codex_skills): provides an on-demand "迁移 Codex 技能" button in the settings panel to discover and link/copy existing skills from ~/.codex/skills to ~/.agents/skills (skipping the .system directory) with migration count feedback. The migration was adjusted from automatic execution at startup to an on-demand manual button in the settings panel, allowing users to choose whether and when to migrate.
- Added dynamic peer dependency discovery for Harness upgrades: aligned_peer_packages and prepare-runtime.ps1 now dynamically inspect @deepseek-ai/dsh package manifest (dependencies and peerDependencies) to automatically discover and align future @deepseek-ai/* modular packages alongside base peers.
- Added domestic npm mirror fallback (https://registry.npmmirror.com): npm_latest_release now automatically falls back to npmmirror when registry.npmjs.org fails or times out, preventing update check hangs in mainland network environments.

### Fixed

- Enhanced legacy agent preset compatibility (ensure_legacy_preset_compatibility): made standard <-> code bi-directional across both runtime presets and user-level ~/.dsh/.agent-presets directories.
- Injected DSH_AGENTS_HOME pointing to ~/.agents and GIT_TERMINAL_PROMPT=0 into the runtime environment to prevent silent terminal prompt deadlocks during git plugin operations.
- Enhanced repair_marketplace_metadata to automatically clean up orphaned lock files and temporary files (such as .pnpm-lock.yaml.tmp).

## [0.1.68] - 2026-09-07

### Fixed

- Fixed the true root cause of the scrollable blank zone under the chat composer: DeepX's v0.1.66 full-screen modal rule `[class*="_overlay"] { top: 40px; height: calc(100vh - 40px); padding: ... }` also matched the Harness's zero-size `.overlayAnchor` (an absolutely-positioned popover anchor inside the conversation scroller), stretching it to 760px and inflating the conversation's scrollable overflow by exactly that amount — producing an ever-scrollable white region below the composer that the sticky input could never dock into (and which the browser never shows). The rule is now scoped with `:not([class*="overlayAnchor"])`; root-level scroll locks added in 0.1.67 are kept as defense.

## [0.1.67] - 2026-09-07

### Fixed

- Fixed outer window scroll exposing white space below the chat input box: Locked `html`, `body`, and `#root` with `overflow: hidden !important` and added a window scroll lock listener to strictly prevent root-level window scrolling. All scrolling is contained within Harness internal scrollports (`.wSkVaW_scrollBody`), ensuring the sticky composer remains locked to the viewport floor without floating or showing blank canvas underneath.
- Fixed legacy `code` agent preset compatibility on session resume / model switch: DeepSeek Harness renamed the default coding preset from `code` to `standard`. Sessions created under previous versions threw `RemoteError: agent-presets: preset "code" not found` when resumed or switched. DeepX now automatically ensures `code` preset compatibility both on disk and in preset resolution fallback.

## [0.1.66] - 2026-09-07

### Fixed

- Fixed settings window collision with titlebar and mutation loops: Full-screen overlay modals (such as DeepSeek Harness settings) are now cleanly padded and bounded below the 40px top titlebar (`[class*="_overlay"]` with `top: 40px`, `height: calc(100vh - 40px)`, and `padding: 24px 20px 20px`). `fitHarnessBelowTitlebar` now excludes modal dialogs/overlays to avoid repeatedly shifting them or triggering layout reflows on tab switches.
- Fixed pnpm detection in plugin marketplace (`dshmarket`): Explicitly exported `PNPM_HOME` pointing to `runtime_dir/bin` in `configure_runtime_environment` so that child processes and plugin managers resolve bundled pnpm reliably without falling back to broken system corepack shims.

## [0.1.65] - 2026-09-04

### Fixed

- Fixed infinite window reloading loop: The toolbar authentication check previously searched `document.body.innerText` with `indexOf` and was invoked from `remount()`, causing it to detect mentions of `dsh web authentication required` inside chat conversation text and repeatedly trigger `location.reload()`. The check is now restricted to initial page load, strictly requires an unmounted SPA (`#root` absent) with exact 401 error text matching, and was removed from the periodic toolbar remount loop.
- Aligned runtime `REQUIRED_DSH_PEERS` list to include all 28 `@deepseek-ai/*` packages required by `@deepseek-ai/dsh@latest` (including `dsh-jobs`, `dsh-attachment`, `dsh-settings`, etc.) ensuring clean updates and compatibility.

## [0.1.64] - 2026-09-04

### Fixed

- Fixed `启动 DeepX 更新安装器失败: 另一个程序正在使用此文件，进程无法访问。 (os error 32)`: During DeepX self-update, the downloaded installer file handle (`tokio::fs::File`) was kept open in the current process when executing `Command::new(&installer).spawn()`. On Windows, the write handle caused an exclusive file lock violation (error 32). The file handle is now explicitly closed before launching the installer, with backoff retry handling for antivirus scanners and version-specific update filenames.

## [0.1.63] - 2026-09-04

### Fixed

- Fixed `dsh web authentication required; reopen the URL printed by dsh web.` in DeepX: When Harness is already running without an active browser session token in logs, navigating to `http://127.0.0.1:3080/` prompted for authentication. DeepX now dynamically mints a signed authentication cookie from the local `~/.dsh/.credentials.yaml` secret and injects it into WebView2, automatically authenticating and reloading into the full DeepSeek Harness Web GUI.

## [0.1.62] - 2026-09-04

### Fixed

- DeepX could fail to open DeepSeek Harness while browser access worked: Harness web authentication returns HTTP 401 Unauthorized for unauthenticated index requests, which caused `healthy()` (`is_success()`) to report false even when Harness was actively running. DeepX then attempted to spawn a duplicate Harness process on port 3080, crashing with `EADDRINUSE`. The health check now recognizes HTTP 401, redirections, and 2xx as healthy, and `navigate_to_harness` extracts tokenized launch URLs when available.
- Window had no top toolbar when Harness failed to launch or during loading: `TOOLBAR_SCRIPT` and `on_page_load` previously only targeted `127.0.0.1:3080`, leaving the local startup/error view borderless with no window controls, drag handle, settings panel, or update button. The toolbar now mounts across both Harness and local pages, with live service connectivity indicators and retry/restart actions on the error panel.

## [0.1.61] - 2026-09-03

### Fixed

- The page-load diagnostic log grew without bound; it is now capped at ~512 KiB (last 400 lines kept) once per launch.

### Docs

- Rewrote the README as a bilingual pair (`README.md` / `README.zh.md`) with current toolbar + update-panel screenshots, and refreshed the stale QA notes.

## [0.1.60] - 2026-09-01

### Fixed

- Dead toolbar buttons after refresh, root-caused with the page-state probe: during a `navigate()` transition the webview can transiently expose a document whose content is the injected script text itself, without Tauri's `__TAURI_INTERNALS__`. A toolbar mounted in that context is permanently deaf (every handler silently returns), yet the old health check only tested whether a toolbar element was connected, so the deaf bar was never replaced. The mount guard now verifies the full health tuple (element connected + owning closure has a working IPC binding + the element is its own + internals resolvable now), tears down every stale toolbar element, and refuses to mount at all while internals are absent — so only a fully wired toolbar can ever exist.

## [0.1.59] - 2026-09-01

### Fixed

- The `toolbar_probe` diagnostic command was unreachable from the harness page: the capability file gates every command explicitly and the new command had no permission entry. Added `allow-toolbar-probe` and extended the probe to report from every watchdog branch (healthy / closure-remount / full mount) so field diagnosis sees which path actually runs.

## [0.1.58] - 2026-09-01

### Fixed

- Toolbar buttons could go dead (panel, window controls, drag all silent) while the bar itself stayed visible. The IPC handle was captured once at injection time, so a toolbar whose closure missed `__TAURI_INTERNALS__` (or whose internals appeared later) stayed mute forever. All handlers now resolve the IPC binding at click time, and the refresh button no longer stays disabled when the reload command resolves without replacing the document.

### Added

- `toolbar_probe` diagnostic command (inactive without a local flag file) reporting toolbar page state for field diagnosis.

## [0.1.57] - 2026-09-01

### Fixed

- The toolbar could vanish for good after clicking refresh (↻). `reload_harness` performs a full `navigate()` to the harness URL; the re-injection races that navigation — the eval dispatched on `PageLoadEvent::Finished` can execute in the doomed previous document (its mount is destroyed with it) or its timers can be lost to a later in-page wipe, and since the script's entry guard trusted a `window` flag rather than the DOM, a re-eval could never recover it. The script's mount guard is now DOM-based (mount only when no live toolbar element exists; nudge `remount()` otherwise) and a main-thread watchdog in Rust re-evaluates the idempotent script every ~1.6s, so any lost injection or wipe heals within one tick. The script also self-guards on hostname/port, making it safe to evaluate at any time.

## [0.1.56] - 2026-09-01

### Fixed

- The top toolbar (window controls − O ×, refresh ↻, update) never rendered at all: the injected toolbar script contained a JavaScript syntax error. Its CSS string literal lost its closing quote — the line had been truncated at 2 000 characters and a "(line truncated to 2000 chars)" marker was written into the source — so the string swallowed the following line, V8 rejected the whole script with "Invalid or unexpected token", and `mountToolbar()` never ran. `on_page_load` still logged `EVAL result=Ok(())` because `webview.eval` only reports asynchronous dispatch, which masked the failure. The corruption entered in commit 3865246, meaning the toolbar was silently dead across 0.1.52–0.1.55; the 0.1.55 entry below (a suspected SPA re-render wipe) was a wrong diagnosis. The CSS is restored in full from the last known-good revision (bb62f53) with the 0.1.54 window-control rules re-applied, and is now written as several short concatenated string literals so no single over-long line can be truncated again.

## [0.1.55] - 2026-09-01

### Fixed

- Superseded by 0.1.56: this release only re-shipped the broken toolbar script with a survival guard added on top. (The SPA re-render wipe described below did not exist — the script never parsed, so nothing was ever mounted for the harness to drop.)

- Original (incorrect) note: the injected top toolbar (window controls − O ×, refresh ↻ update) could vanish after launch: the harness is a client-side SPA that renders into #root after load, and its post-injection re-render could drop the toolbar <header>/<style> from the DOM. Unlike the removed winbar pill, the in-page toolbar had no survival guard, so once dropped it stayed gone (on_page_load only re-fires on a full navigation, not on SPA re-renders). The toolbar is now re-asserted on a short interval and on DOM mutations, so the harness can no longer remove the window controls.

## [0.1.54] - 2026-09-01

### Changed

- Window controls (− O ×) now render **inside** the top toolbar row, on the same line as the refresh (↻) and update buttons, instead of in a separate native window docked at the top-right. The main window is undecorated (decorations false), so its toolbar row is the custom titlebar; the controls belong in-page, wired to the existing window_action command (minimize / toggle_maximize / close). This removes the popping-out / separated-from-the-bar look: the buttons are styled to match the drag area and sit flush in the row.

### Removed

- The separate native winbar window and its machinery: sync_winbar, window_dpi_scale, the WINBAR_URL const, the set_winbar_size command, and the WebviewWindowBuilder that created the winbar. The harness SPA scrubbed the #winbar URL hash and did a cross-document navigation that wiped the injected pill, so a standalone window could never reliably host the controls.

## [0.1.53] - 2026-09-01

### Fixed

- Winbar window-controls pill (again): the harness SPA re-renders after the pill is injected and replaces the whole document, which also removed the `MutationObserver` that was meant to keep the pill mounted. The pill is now re-asserted on a short interval that survives a full document swap, so the harness can no longer drop it.

## [0.1.52] - 2026-09-01

### Changed

- Winbar (window controls − O ×) now renders consistently inside the top toolbar row: removed the JS `fit()` override that shrank the winbar to the harness page's content size (110×41) instead of the pinned 138×51 physical size Rust computes from the real DPI, so the strip matches the toolbar row height and docks flush to the top-right. The pill is also re-asserted via a `MutationObserver` so the harness SPA's post-injection re-render can no longer drop the overlay.

## [0.1.51] - 2026-09-01

### Fixed

- the winbar height now tracks the real OS display scale via GetDpiForWindow
  instead of Tauri's scale_factor(), which can report 1.0 for a zoomed webview;
  this keeps the window-controls row the same physical height as the toolbar it
  docks into.

## [0.1.50] - 2026-09-01

### Fixed

- the winbar window controls no longer float as a short strip above the
  toolbar: its height now matches the injected toolbar row's physical height
  (40 CSS px × main scale factor) and it draws the same bottom border, so the
  two segments read as one continuous top bar instead of a detached pill.

## [0.1.49] - 2026-09-01

### Added

- 任务栏图标右键菜单（JumpList）新增「重启 DeepSeek Harness」：点击后以
  `--restart-harness` 再次启动本程序，由 single-instance 回调路由到已运行实例，
  停止并重新拉起本地 Harness 服务。
- 托盘图标菜单新增同名「重启 DeepSeek Harness」入口，隐藏到托盘时也能一键重启。

## [0.1.48] - 2026-08-31

### Fixed

- pin the winbar webview zoom to 1.0: WebView2 paints the tiny window at
  ~1.24x, overflowing 138 CSS px into ~171 physical px and clipping the close
  button. With CSS px == physical px all three window controls fit.
- grant `core:window:allow-set-size`/`allow-set-position` so the winbar can
  resize and re-dock at runtime.

## [0.1.47] - 2026-08-31

### Fixed

- the winbar webview renders at a viewport wider than its HWND, clipping the
  close button. The winbar script now measures its real viewport and resizes
  the window to match (`set_winbar_size` command), and `sync_winbar` docks using
  the winbar's actual width.
- use `—` `O` `X` glyphs for the window controls; the box-drawing glyphs are
  unreliable in the WebView2 font stack.

## [0.1.46] - 2026-08-31

### Changed

- temporary: give the winbar buttons distinct solid backgrounds so the app's
  real button layout can be probed by pixel capture while diagnosing why only
  two glyphs paint.

## [0.1.45] - 2026-08-31

### Fixed

- declarative `.truncate(false)` on the diagnostic log handles so clippy
  (`-D warnings`) accepts the append-mode open.

## [0.1.44] - 2026-08-31

### Fixed

- append (not overwrite) the diagnostic log so the full page-load trace survives.
- evaluate the toolbar script in the winbar on any finished load (no URL
  filter), covering harness hash/URL rewrites that previously skipped the eval.

## [0.1.43] - 2026-08-31

### Fixed

- use `{:?}` for the diagnostic page-load log (PageLoadEvent has no Display).

## [0.1.42] - 2026-08-31

### Added

- temporary diagnostics: log every page load (label/url/event), eval result and
  the winbar's current URL to `%LOCALAPPDATA%\deepx-onload.log` while the winbar
  rendering issue is tracked down.

## [0.1.41] - 2026-08-31

### Fixed

- the harness page scrubs unknown URL hashes, so the winbar never saw the
  `#winbar` branch. The winbar window is now flagged directly from Rust
  (`window.__deepxWinbarMode=true;`) before the toolbar script runs, so the
  window controls render regardless of the page URL.

## [0.1.40] - 2026-08-31

### Fixed

- use `×` (U+00D7) for the close button glyph; `✕` (U+2715) is not present
  in the WebView2 font stack and painted nothing.

## [0.1.39] - 2026-08-31

### Fixed

- drop the unused `built` binding left over from the winbar builder rework.

## [0.1.38] - 2026-08-31

### Fixed

- sync the native winbar through the WebviewWindow handle (Tauri v2 has no
  `WebviewWindow::window()` accessor; all needed APIs live on the fused type).

## [0.1.37] - 2026-08-31

### Fixed

- winbar window builds with the correct Tauri v2 API (`build()` takes no
  manager argument; sync through the inner Window handle).

## [0.1.36] - 2026-08-31

### Fixed

- Window controls (minimize/maximize/close) now live in their own small native
  window (`winbar`) docked to the top-right of the main window. The window is
  OS-composited and follows the main window on move/resize and hides with it,
  so the controls stay visible and clickable even where the embedded WebView
  fails to paint right-anchored page UI.

## [0.1.35] - 2026-08-31

### Fixed

- Window controls are now rendered in their own right-pinned bar
  (`deepx-winbar`) with `!important` layout rules, instead of relying on
  the toolbar's flex row. The bar is re-appended to the end of the document
  whenever the Harness UI re-renders, so minimize/maximize/close stay
  visible and clickable regardless of page styles.

## [0.1.34] - 2026-08-31

### Fixed

- The Harness page draws its own right-anchored top chrome at z25, which
  covered the titlebar's window controls (minimize/maximize/close) even
  after the 40px offset. The toolbar now sits at the top z-index, so the
  controls are always visible and clickable; a MutationObserver keeps
  re-applying the 40px offset when the Harness UI re-renders, so the
  sidebar and content stay below the titlebar.

## [0.1.33] - 2026-08-31

### Changed

- Slim installer: the full Harness runtime is no longer bundled into the
  setup exe (it was ~300MB, so every update re-downloaded everything). The
  runtime is now released as a separate `deepx-runtime-v<version>.zip` asset
  and only fetched when a machine is missing it; app updates are now a small
  download. WebView2 switches to a download bootstrapper (only fetched when
  absent) instead of bundling the full runtime.

## [0.1.32] - 2026-08-31

### Fixed

- The Harness page's own top chrome (panel tabs / session status) rendered in
  the plugin layer above the titlebar, hiding the window controls and making
  the window impossible to close. The injected script now offsets every fixed
  top-pinned element of the Harness UI down by 40px, so the sidebar and all
  content start below the titlebar; window buttons (minimize/maximize/close)
  are back at the standard top-right corner and always visible/clickable.

## [0.1.31] - 2026-08-30

### Fixed

- Window control buttons (minimize/maximize/close) on the custom titlebar
  were hidden under the Harness page's own top chrome (its panel tabs render
  in the plugin layer above the titlebar). Moved them to the left side of the
  titlebar where nothing overlaps, so they stay visible and clickable.

## [0.1.30] - 2026-08-30

### Changed

- The window is now undecorated (`decorations: false`): 刷新 / 更新 / 应用名 /
  最小化 / 最大化 / 关闭 全部放进第一行顶栏（自绘标题栏），一行完成所有
  窗口操作。标题栏 z-index 仍在插件宿主层之下，任何置顶插件都不会被遮住。

## [0.1.29] - 2026-08-29

### Fixed

- In-app 更新 now runs the NSIS installer with `/S /R` (silent install, then
  relaunch the fresh app). GUI-mode installs could hang on the
  remove-previous-version step and leave the main exe locked when app
  processes were still running, so updates never actually applied; the silent
  flow skips every dialog and replaces the exe cleanly.
- The injected top toolbar script had a JS syntax error (unterminated string
  in its CSS line) since 0.1.27, so 刷新 / 更新 never actually rendered on the
  Harness page; the script was regenerated and is now syntax-verified
  (`node --check`) before embedding.

## [0.1.28] - 2026-08-28

### Changed

- Revert the standalone update window: clicking 更新 in the top toolbar now
  toggles the in-page update panel (DeepX / Harness / 插件市场 status, action
  buttons, progress bar with downloaded size / speed / ETA) exactly like the
  original 0.1.23 panel.
- Remove the separate "update" window entirely: WebviewWindowBuilder, the
  open_update_window command and its permission, the update.html entry and
  files, and the capability window entry.

## [0.1.27] - 2026-08-28

### Added

- Bring back the top toolbar (刷新 / 更新 / drag region) that hosted the
  refresh and update actions, restored as a non-covering bar: it sits at
  z-index 20, BELOW the plugin host layer (z-index 25, pointer-events:none),
  so any plugin UI pinned to the top of the Harness page renders above the
  toolbar and stays clickable regardless of position.

## [0.1.26] - 2026-08-28

### Changed

- The DeepX update window now also shows the remaining download time
  (e.g. 「剩余 22 分钟」) and reports slow speeds in KB/s, matching the
  classic download-progress readout (speed - downloaded/total, ETA).

## [0.1.25] - 2026-08-28

### Added

- Dedicated "DeepX 更新" window opened from the tray menu (「更新 DeepX」)
  showing download progress: progress bar, downloaded/total size and
  per-second speed (MB/s) while the installer downloads.
- DeepX updates are now streamed to disk with granular `deepx-update-progress`
  events instead of buffering the whole installer in memory.

## [0.1.24] - 2026-08-28

### Fixed

- Use the native window titlebar (`decorations: true`) and remove the injected DOM titlebar overlay that covered plugin UI anchored to the top of the Harness page — most visibly the `dsh-better-sidebar` expand/collapse toggle in the top-right corner, which made the sidebar workbench appear absent.
- Drop the overlay-related verification scripts and their CI/release steps.

### Changed

- Harness reload is now available from the system tray menu (「刷新页面」).

## [0.1.20] - 2026-08-27

### Fixed

- Restore Rust formatting so the full Windows quality gate passes before release.
## [0.1.19] - 2026-08-27

### Fixed

- Map the Harness title-bar maximize button to the supported `toggle_maximize` window action.
- Report an unavailable Tauri IPC bridge instead of treating a click as successful with no visible result.
## [0.1.18] - 2026-08-27

### Fixed

- Normalize all custom window actions to the supported Tauri error type.
## [0.1.17] - 2026-08-27

### Fixed

- Use the supported Tauri maximize and unmaximize APIs for the custom window controls.
## [0.1.16] - 2026-08-27

### Fixed

- Make the custom DeepX title bar draggable on both the startup shell and the Harness page.
- Route minimize, maximize, and close through an allowed application command so the controls work on remote Harness content.
- Check update versions once when the panel is mounted; only the panel refresh button checks again.
- Show `安装` for missing components, `更新` for installed outdated components, and no action button when current.
## [0.1.15] - 2026-08-26

### Fixed

- Preserve pnpm metadata in normal plugin profiles instead of deleting it on
  every Harness launch, avoiding unnecessary complete dependency relinks.
- Serialize pnpm operations for each profile so concurrent plugin installs do
  not race while replacing native dependencies such as `node-pty` on Windows.

## [0.1.14] - 2026-08-26

### Fixed

- Restore valid locked versions for the Windows build dependencies, allowing
  GitHub Actions to resolve the Rust dependency graph and build the installer.

## [0.1.13] - 2026-08-26

### Changed

- Replace the native Windows title bar with a compact integrated bar that keeps
  page reload and window controls on the same row in both startup and Harness.

### Fixed

- Route the top-left page reload through the Harness navigation command so the
  DeepX controls are mounted again after restarting the page.

## [0.1.12] - 2026-08-26

### Fixed

- Update `dshmarket` with `dshmarket@latest` and bypass pnpm’s release-age cache,
  so the action installs the newest published marketplace release instead of
  retaining the old lockfile version.
- Always update Harness from the newest published npm release.

### Changed

- Remove the update-channel selector and pnpm update row from the user-facing
  panel; pnpm remains bundled and configured during initialization.

## [0.1.9] - 2026-08-26

### Fixed

- Apply the Rustfmt correction required for the portable marketplace metadata repair release.

## [0.1.8] - 2026-08-26

### Fixed

- Remove pnpm workspace state with machine-specific CI paths from bundled and repaired marketplace profiles.

## [0.1.7] - 2026-08-26

### Fixed

- Treat an incomplete dshmarket installation as missing so first-run repair runs instead of opening a broken marketplace.

## [0.1.6] - 2026-08-26

### Fixed

- Run first-run marketplace initialization for upgrades whose Harness runtime is already present but whose marketplace is missing.

## [0.1.5] - 2026-08-25

### Fixed

- Repair legacy pnpm metadata before Harness or marketplace commands so upgrades from v0.1.3 remain portable.

## [0.1.4] - 2026-08-25

### Fixed

- Strip machine-specific pnpm metadata from the bundled marketplace profile so copied installations can update plugins on any Windows account.
- Remove stale pnpm metadata again when seeding a marketplace profile during first-run initialization.

## [0.1.3] - 2026-08-25

### Fixed

- Bundle a private pnpm runtime and force Harness plugin commands to use it instead of a broken system Corepack installation.
- Preinstall the plugin marketplace during first-run initialization, so a new Windows installation does not need to set up pnpm before using it.
- Show current and latest versions for DeepX, Harness, the plugin marketplace, and pnpm in the update panel.

## [0.1.2] - 2026-08-25

### Fixed

- Keep first run offline: copy the bundled Harness runtime instead of running an update.
- Align all Harness peer packages to the installed Harness version and verify the web service before packaging.
- Show the actual Harness startup failure in the local log instead of masking it as a timeout.

### Changed

- Reduce initialization to concise status text.

## [0.1.1] - 2026-08-25

### Changed

- Bundle the tested Node.js and official DeepSeek Harness runtime so first run copies local files instead of downloading or running npm.
- Install WebView2 from the bundled offline installer when it is missing.
- Keep the DeepX, Harness, and plugin-market update controls mounted above the Harness settings area.
- Update DeepX directly to the newest GitHub Release installer.
- Replace the product icon with a black, flat whale-tail mark with an abstract negative-space X scar.

### Fixed

- Copy the bundled runtime on a blocking worker so first-run setup does not freeze the app window.
## [0.1.0] - 2026-08-25

### Added

- Clean-room Tauri desktop shell for the official DeepSeek Harness runtime.
- First-run installation of the official Harness runtime and Node.js host.
- Direct launch of Harness on `127.0.0.1:3080`.
- Overlay controls for refresh, Harness updates, update channels, and the plugin marketplace.
- Persistent `latest` and `next` npm update channels.
- Shared use of the default `~/.dsh` profile and one-time migration from the former private profile.
- Single-instance activation, close-to-tray behavior, and tray restoration.
- Suppression of default-browser handoff while Harness remains inside DeepX.

### Security

- Local IPC permissions are scoped to the DeepX window and `127.0.0.1:*`.