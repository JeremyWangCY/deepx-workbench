# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
