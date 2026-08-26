<p align="center"><img src="assets/deepx-icon.png" width="144" alt="DeepX Workbench icon"></p>

# DeepX Workbench

[![CI](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml)
[![Release](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/release.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeepX Workbench is an independent, clean-room desktop shell for the official
DeepSeek Harness runtime. It is **not a fork** of `deepseek-harness-desktop`.
It has its own minimal Vite frontend and its own Rust/Tauri host; the only
compatibility surface is the official Harness runtime and CLI.

Compatibility surface (official Harness only):

- local Harness HTTP service at `127.0.0.1:3080`
- `dsh --profile web --no-open --port 3080`
- `dsh plugin --profile web add dshmarket`
- the official npm package `@deepseek-ai/dsh`

## Install

Download the Windows x64 NSIS installer from
[Releases](https://github.com/JeremyWangCY/deepx-workbench/releases/latest).
The current asset is named `DeepX.Workbench_0.1.11_x64-setup.exe`.

The Windows installer contains a tested private Node.js runtime and the official
DeepSeek Harness runtime. On first run DeepX copies the local Harness, private pnpm runtime, and preinstalled marketplace into its
app-data folder; it does **not** download Node.js or run npm. Once the copy
finishes, Harness opens directly in the window.

No dependency installation runs on normal launches. The private pnpm environment and
plugin marketplace are ready after first-run setup; later Harness and marketplace
updates are explicit user actions and need an internet connection.

## Using Harness updates and the plugin marketplace

A small control is injected into the Harness page in the lower-left area,
above the settings gear. It is deliberately **not** inside the Harness
settings page.

- **刷新状态** - the small header button rechecks current/latest versions for
  DeepX, Harness, and the plugin marketplace.
- **更新 Harness** - updates the newest published Harness release on demand
  (stop service, install Harness, relaunch, navigate back). Probe progress is
  shown inline.
- **安装 / 更新插件市场** - installs or updates `dshmarket` to the newest
  published npm release using the official `dsh plugin` CLI and DeepX’s bundled
  pnpm runtime.

## Design goals

- minimal startup surface, direct to Harness
- no default-browser handoff; the Harness surface stays inside DeepX
- one app instance; shortcuts restore a minimized or tray-hidden window
- the default `~/.dsh` web profile, so existing Harness plugins are shared
- no repeated dependency installs
- explicit, user-triggered updates to the newest published releases
- bundled pnpm and a ready-to-use marketplace on first run
- preserve the Harness profile and user-installed plugins (never touched)

## Development

Install Node.js and pnpm, then:

```bash
pnpm install
pnpm tauri dev
```

To build installers:

```bash
pnpm tauri build
```

Contribution commands, commit conventions, and release steps are documented in
[CONTRIBUTING.md](CONTRIBUTING.md). Security reports are handled through
[SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)

## Quality assurance

Manual verification notes live in [`docs/QA.md`](docs/QA.md).
