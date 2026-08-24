# DeepX Workbench

DeepX Workbench is an independent, clean-room desktop shell for the official
DeepSeek Harness runtime. It is **not a fork** of `deepseek-harness-desktop`.
It has its own minimal Vite frontend and its own Rust/Tauri host; the only
compatibility surface is the official Harness runtime and CLI.

Compatibility surface (official Harness only):

- local Harness HTTP service at `127.0.0.1:3080`
- `dsh web --port 3080`
- `dsh plugin --profile web add dshmarket`
- the official npm package `@deepseek-ai/dsh`

## Install

Grab the Windows installer (NSIS `DeepX Workbench ...-setup.exe`) from the
latest release. On first run DeepX installs its private Node runtime plus the
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

- **刷新 / 更新 Harness** - checks the npm registry for the latest `@deepseek-ai/dsh`,
  shows installed vs latest, and runs the update on demand (stop service,
  install latest, relaunch, navigate back). Probe progress is shown inline.
- **安装 / 更新插件市场** - one-click install/update of the `dshmarket`
  plugin using the official `dsh plugin` CLI, so users can then explore
  Harness extensions from the marketplace.

## Design goals

- minimal startup surface, direct to Harness
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

## QA

Manual verification notes and screenshots live in `output/QA_NOTES.md`.
