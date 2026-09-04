<p align="center"><img src="assets/deepx-icon.png" width="144" alt="DeepX Workbench icon"></p>

# DeepX Workbench

**[中文版](README.zh.md)**

[![CI](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml)
[![Release](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/release.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeepX Workbench is an independent, clean-room desktop shell for the official
DeepSeek Harness runtime. It is **not a fork** of `deepseek-harness-desktop`:
it has its own minimal Vite frontend and its own Rust/Tauri host. The only
compatibility surface is the official Harness runtime and CLI:

- local Harness HTTP service at `127.0.0.1:3080`
- `dsh --profile web --no-open --port 3080`
- `dsh plugin --profile web add dshmarket`
- the official npm package `@deepseek-ai/dsh`

## What you get

A borderless desktop window that hosts Harness directly, with a small top
toolbar: drag-to-move title area, refresh, an update panel for DeepX /
Harness / marketplace versions with one-click updates, and standard minimize,
maximize, and hide-to-tray controls. A tray icon offers show / refresh /
restart / quit, and the app runs as a single instance.

## Install

Download the Windows x64 NSIS installer from
[Releases](https://github.com/JeremyWangCY/deepx-workbench/releases/latest)
(asset pattern `DeepX.Workbench_<version>_x64-setup.exe`). It is a per-user
install and needs no administrator rights.

The installer ships a tested private Node.js runtime plus the official
DeepSeek Harness runtime. On first run DeepX copies the local Harness, the
private pnpm runtime, and the preinstalled marketplace into its app-data
folder — it does **not** download Node.js or run npm at install time. Once the
copy finishes, Harness opens directly in the window.

No dependency installation runs on normal launches. The private pnpm
environment and plugin marketplace are ready after first-run setup; later
Harness and marketplace updates are explicit user actions and need an internet
connection.

## Usage

Launch DeepX Workbench from the Start Menu or desktop shortcut. The window
opens directly into Harness at `127.0.0.1:3080`.

## Design goals

- Minimal startup surface, straight into Harness.
- No default-browser handoff; the Harness surface stays inside DeepX.
- One app instance; shortcuts restore a minimized or tray-hidden window.
- The default `~/.dsh` web profile, so existing Harness plugins are shared.
- No repeated dependency installs.
- Explicit, user-triggered updates to the newest published releases.
- Bundled pnpm and a ready-to-use marketplace on first run.
- Preserve the Harness profile and user-installed plugins (never touched).
- A toolbar that survives page reloads: re-injected on every page load, with a
  main-thread watchdog re-asserting it about every 1.6 s, a health check that
  only accepts a fully wired bar (connected element owned by a closure with a
  live IPC binding), and click-time IPC resolution so buttons never go deaf.

## Development

Install Node.js, pnpm, and Rust stable, then:

```bash
pnpm install
pnpm prepare:runtime
pnpm tauri dev
```

To build installers:

```bash
pnpm tauri build
```

Release builds run on tags (`v*`) through `.github/workflows/release.yml`:
version check → dependency install → runtime prep → `pnpm typecheck` +
`cargo clippy -- -D warnings` → NSIS build → runtime archive → GitHub
Release. Contribution commands, commit conventions, and release steps are
documented in [CONTRIBUTING.md](CONTRIBUTING.md). Security reports are handled
through [SECURITY.md](SECURITY.md).

## Docs

- [CHANGELOG.md](CHANGELOG.md) — per-version notes.
- [docs/QA.md](docs/QA.md) — manual verification notes.

## License

[MIT](LICENSE)
