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
The `0.1.0` asset is named `DeepX Workbench_0.1.0_x64-setup.exe`.

On first run DeepX installs its private Node runtime plus the
official Harness package automatically. This is a **one-time** step and takes a
few minutes (it pulls the large dependency tree). Once it finishes, Harness
opens directly in the window.

No dependency installation runs on normal launches after that. If an existing
Harness runtime is already present in the app-data folder, startup goes
straight to the service boot with no install step.

## Using Harness updates and the plugin marketplace

A small control is injected into the Harness page in the lower-left area,
above the settings gear. It is deliberately **not** inside the Harness
settings page.

- **刷新状态** - the small header button rechecks installed/latest versions and
  marketplace status.
- **更新 Harness** - runs the selected-channel update on demand (stop service,
  install Harness, relaunch, navigate back). Probe progress is shown inline.
- **更新通道** - chooses the npm `latest` or `next` dist-tag. The choice is
  persisted in DeepX application data and used for both update checks and
  installs; `latest` remains the default.
- **安装 / 更新插件市场** - one-click install/update of the `dshmarket`
  plugin using the official `dsh plugin` CLI, so users can then explore
  Harness extensions from the marketplace.

## Design goals

- minimal startup surface, direct to Harness
- no default-browser handoff; the Harness surface stays inside DeepX
- one app instance; shortcuts restore a minimized or tray-hidden window
- the default `~/.dsh` web profile, so existing Harness plugins are shared
- no repeated dependency installs
- explicit, user-triggered Harness updates
- explicit, user-triggered marketplace install
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
