<p align="center"><img src="assets/deepx-icon.png" width="144" alt="DeepX Workbench 图标"></p>

# DeepX Workbench

**[English](README.md)**

[![CI](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/ci.yml)
[![Release](https://github.com/JeremyWangCY/deepx-workbench/actions/workflows/release.yml/badge.svg)](https://github.com/JeremyWangCY/deepx-workbench/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

DeepX Workbench 是一个面向 Windows 的轻量 DeepSeek Harness 桌面壳。它把官方 Harness Web 界面直接托管在 Tauri/WebView2 窗口里，并负责本地运行时、进程生命周期、更新、恢复和常用维护操作。

DeepX 不修改 Harness 的业务逻辑，也不替代 Harness。本项目的目标是把“启动、连接、更新和维护 Harness”做成稳定的桌面体验。

## 主要能力

- **直接进入 Harness**：不跳转默认浏览器，Harness 始终留在 DeepX 窗口内。
- **动态本地端口**：DeepX 启动 Harness 时自动选择可用的 loopback 端口，不再依赖固定 `3080`；升级时也能重新识别旧的固定端口实例。
- **进程监督与自动恢复**：Harness 异常退出或连续失去健康状态时，DeepX 会尝试恢复，而不是让桌面壳直接失效。
- **安全更新 Harness**：新版运行时先在 staging 中更新和校验，启动失败时自动回滚到上一版。
- **DeepX 自更新**：从 GitHub Release 获取最新 Windows 安装包，由用户主动触发。
- **连接恢复**：Harness 重启后，DeepX 会尽量恢复到之前的页面路由；不会改写 Harness 的 session 存储。
- **日志脱敏**：Harness 启动 URL 中的认证 token 不会原样写入 DeepX 日志。
- **系统托盘**：关闭主窗口时保留后台运行，可从托盘重新显示、刷新、重启或退出。

## 安装

目前主要支持 **Windows x64**。

从 [GitHub Releases](https://github.com/JeremyWangCY/deepx-workbench/releases/latest) 下载：

`DeepX.Workbench_<版本>_x64-setup.exe`

安装为当前用户应用，通常不需要管理员权限，也不要求系统预装 Node.js、npm 或 pnpm。

正式 Release 同时提供一个独立的 `deepx-runtime-v<版本>.zip`。这样 Windows 安装包可以保持较小，而完整的 Node.js / pnpm / Harness 运行环境由 DeepX 在首次启动时按当前版本获取。

## 首次启动

第一次启动时，如果应用数据目录里还没有有效运行时，DeepX 会从**同版本 GitHub Release** 下载并解压 `deepx-runtime-v<版本>.zip`，然后准备插件市场并启动 Harness。因此首次启动需要能够访问 GitHub Release。

启动页只保留一个“正在准备...”状态，直到 Harness 真正接管 WebView。

运行时准备完成后，之后的正常启动不会重复下载或安装依赖；只有显式更新或运行时损坏恢复时才会进入相应的更新 / 修复流程。

## 界面

顶部工具栏提供：

- 刷新当前页面
- DeepX / Harness / 插件市场更新入口
- DeepX 设置
- 最小化、最大化、关闭

### 设置

设置页只保留会变化或可以操作的内容：

**连接**
- Harness 运行状态
- 当前本地地址
- 重启 Harness

**文件**
- 配置目录
- 插件目录
- 技能目录
- 启动日志

**维护**
- 修复插件环境
- 迁移 Codex 技能

底部只显示 DeepX 和 Harness 当前版本。

## 数据与目录

DeepX 尽量遵循 Harness 自己的目录约定：

- Harness 用户配置 / Web profile：`~/.dsh`
- Agent skills：`~/.agents/skills`
- DeepX 私有运行时、更新 staging、运行日志：DeepX 的应用数据目录

DeepX 不会为了页面恢复去修改 Harness 的 session 文件。

## 更新策略

### DeepX

工具栏中的“更新”面板会检查 GitHub Release。只有用户主动点击更新时，DeepX 才下载并启动新版安装器。

### Harness

Harness 更新采用事务式流程：

1. 复制当前运行时到 staging
2. 在 staging 中更新 Harness
3. 校验关键文件和版本一致性
4. 停止旧 Harness 并激活新版
5. 验证新版是否能启动
6. 失败则自动回滚并尝试重新启动上一版

这样可以尽量避免“更新到一半把当前运行环境破坏掉”。

## 故障排查

遇到 Harness 无法连接或启动异常时，可以按这个顺序处理：

1. 打开 **设置 → 启动日志**
2. 尝试 **重启 Harness**
3. 如涉及插件，尝试 **修复插件环境**
4. 需要更细的前端信息时可使用 F12 打开 WebView2 开发者工具
5. 查看 `harness-supervisor.log` 了解 DeepX 的进程恢复和回滚事件

如果问题仍然存在，请在 GitHub 提交 issue，并附上 DeepX 版本、Harness 版本和相关日志片段。不要公开粘贴认证 token 或其他私密凭据。

## 开发

需要 Node.js、pnpm 和 Rust stable：

```bash
pnpm install
pnpm prepare:runtime
pnpm tauri dev
```

构建：

```bash
pnpm tauri build
```

常用质量检查：

```bash
pnpm typecheck
cargo fmt --all --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

CI 会在 Pull Request 和 master 上执行运行时准备、格式检查、前端类型检查、Rust clippy、library tests 和完整 Tauri 构建。

正式发布由 `v*` tag 触发 `.github/workflows/release.yml`，生成 Windows NSIS 安装包和 runtime zip，并发布到 GitHub Releases。

## 相关文档

- [CHANGELOG.md](CHANGELOG.md) — 版本变更
- [CONTRIBUTING.md](CONTRIBUTING.md) — 开发与发布流程
- [SECURITY.md](SECURITY.md) — 安全问题报告
- [docs/QA.md](docs/QA.md) — QA / 手工验证记录

## License

[MIT](LICENSE)
