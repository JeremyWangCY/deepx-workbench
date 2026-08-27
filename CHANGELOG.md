# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
