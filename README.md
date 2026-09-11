<p align="center"><img src="assets/deepx-icon.png" width="144" alt="DeepX Workbench icon"></p>

# DeepX Workbench

**[简体中文](README.zh.md)**

[![CI](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml)
[![Release](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/release.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeepX Workbench is a lightweight Windows desktop host for DeepSeek Harness. It keeps the official Harness Web UI inside a Tauri/WebView2 window and handles the local runtime, process lifecycle, updates, recovery, and common maintenance tasks around it.

DeepX does not replace or modify Harness business logic. Its job is to make starting, connecting to, updating, and maintaining Harness feel like a reliable desktop application.

## Highlights

- **Harness stays inside DeepX** — no default-browser handoff.
- **Dynamic local endpoint** — new Harness processes use an automatically selected loopback port instead of relying on fixed port `3080`; DeepX can still rediscover older fixed-port instances during upgrades.
- **Process supervision and recovery** — DeepX watches the Harness it owns and can recover from unexpected exits or repeated health failures.
- **Transactional Harness updates** — updates are prepared and validated in staging, then activated; a failed launch automatically rolls back to the previous runtime.
- **DeepX self-update** — the latest Windows installer is fetched from GitHub Releases only when the user requests an update.
- **Route restore** — after a Harness restart, DeepX can return to the previous WebView route without modifying Harness session storage.
- **Redacted logs** — Harness authentication tokens are not persisted in DeepX startup logs.
- **System tray behavior** — closing the main window keeps the app available in the tray, where it can be shown, refreshed, restarted, or exited.

## Install

DeepX Workbench currently targets **Windows x64**.

Download the latest installer from [GitHub Releases](https://github.com/JeremyWangCY/deepx-workbench/releases/latest):

`DeepX.Workbench_<version>_x64-setup.exe`

It is installed for the current user and normally does not require administrator privileges. A system-wide Node.js, npm, or pnpm installation is not required.

Each release also publishes a separate `deepx-runtime-v<version>.zip`. This keeps the Windows installer small while allowing DeepX to provision the full Node.js / pnpm / Harness runtime on first launch.

## First launch

If no valid runtime exists in the application-data directory, DeepX downloads and extracts `deepx-runtime-v<version>.zip` from the **matching GitHub Release**, then prepares the marketplace and starts Harness. First launch therefore needs access to GitHub Releases.

The startup surface stays on one “preparing” state until Harness actually takes over the WebView.

After the runtime has been prepared, normal launches do not redownload or reinstall dependencies. Network access is needed again only for explicit updates or recovery paths that require a fresh runtime.

## UI

The top toolbar provides:

- page refresh
- DeepX / Harness / marketplace updates
- DeepX settings
- minimize, maximize, and close controls

### Settings

The settings panel focuses only on changing or actionable information:

**Connection**
- Harness runtime status
- current local endpoint
- restart Harness

**Files**
- configuration directory
- plugin directory
- skills directory
- startup log

**Maintenance**
- repair plugin environment
- migrate Codex skills

A compact footer shows the current DeepX and Harness versions.

## Data and directories

DeepX follows Harness conventions where possible:

- Harness user configuration / Web profile: `~/.dsh`
- Agent skills: `~/.agents/skills`
- DeepX private runtime, update staging, and runtime logs: the DeepX application-data directory

DeepX does not modify Harness session files just to restore the last page route.

## Update model

### DeepX

The Update panel checks GitHub Releases. DeepX downloads and launches a newer installer only after an explicit user action.

### Harness

Harness updates are transactional:

1. copy the current runtime into staging
2. update Harness inside staging
3. validate required files and version alignment
4. stop the old Harness and activate the staged runtime
5. verify that the new Harness starts correctly
6. automatically roll back and relaunch the previous runtime if validation fails

This keeps a failed update from corrupting the active runtime in place.

## Troubleshooting

If Harness does not connect or starts incorrectly:

1. open **Settings → Startup log**
2. try **Restart Harness**
3. for plugin-related issues, try **Repair plugin environment**
4. use F12 for WebView2 developer tools when frontend diagnostics are needed
5. inspect `harness-supervisor.log` for DeepX process recovery and rollback events

When opening a GitHub issue, include the DeepX version, Harness version, and relevant log excerpts. Do not post authentication tokens or other private credentials.

## Development

Install Node.js, pnpm, and Rust stable:

```bash
pnpm install
pnpm prepare:runtime
pnpm tauri dev
```

Build:

```bash
pnpm tauri build
```

Useful quality checks:

```bash
pnpm typecheck
cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

CI runs runtime preparation, formatting checks, frontend type checking, Rust clippy, library tests, and a full Tauri build on pull requests and master.

A `v*` tag triggers `.github/workflows/release.yml`, which builds the Windows NSIS installer and runtime archive and publishes them to GitHub Releases.

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — per-version changes
- [CONTRIBUTING.md](CONTRIBUTING.md) — development and release workflow
- [SECURITY.md](SECURITY.md) — security reporting
- [docs/QA.md](docs/QA.md) — QA and manual verification notes

## License

[MIT](LICENSE)
