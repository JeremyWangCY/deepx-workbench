# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
