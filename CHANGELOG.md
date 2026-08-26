# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
